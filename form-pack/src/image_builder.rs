use std::path::{Path, PathBuf};
use std::process::Command;
use axum::extract::Path as AxumPath;
use axum::{routing::post, Json, Router};
use serde_json::Value;
use std::io::Write;
use serde::{Serialize, Deserialize};
use crate::formfile::{BuildInstruction, Entrypoint, EnvScope, EnvVariable, Formfile, User};
use log::{info, error};

pub const IMAGE_PATH: &str = "/img/jammy-server-cloudimg-amd64.raw";

pub struct VirtCustomize {
    commands: Vec<String>
}

impl VirtCustomize {
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }

    pub fn run_command(mut self, command: &str) -> Self {
        self.commands.push(
            format!(r#"--run-command '{command}'"#)
        );

        self
    }

    pub fn ssh_keygen(self) -> Self {
        let command = format!("ssh-keygen -A");
        self.run_command(&command)
    }

    pub fn run_script(mut self, script: &str) -> Self {
        self.commands.push(
            format!("--run {script}")
        );
        self
    }

    pub fn mkdir(mut self, path: &str) -> Self {
        self.commands.push(format!("--mkdir {path}"));
        self
    }

    pub fn copy_in(mut self, from: &str, to: &str) -> Self {
        self = self.mkdir(to);
        self.commands.push(
            format!("--copy-in {from}:{to}")
        );
        self
    }

    pub fn install(mut self, packages: &[String]) -> Self {
        let packages: String = packages.join(",");
        self.commands.push(
            format!("--install {packages}")
        );
        self
    }

    pub fn ssh_inject(mut self, user:&User) -> Self {
        let username = user.username();
        for key in user.ssh_authorized_keys() {
            self.commands.push(
                format!(r#"--ssh-inject {username}:string:'{key}'"#));
        }
        self
    }
    
    pub fn useradd(mut self, user: &User) -> Self {
        let username = user.username();
        let password = user.passwd();
        let password = &password.replace('$', r"\$").to_string();
        let mut command = format!("useradd -m -s /bin/bash"); 
        if user.sudo() && !user.disable_root() {
            command.push_str(" -g sudo");
        }

        if !user.groups().is_empty() {
            let groups = user.groups().join(",");
            command.push_str(&format!(" -G {groups}"))
        }

        if !user.chpasswd_expire() {
            command.push_str(" -K PASS_MAX_DAYS=-1");
        }

        command.push_str(&format!(r#" -p "{password}""#));

        command.push_str(&format!(" {username}"));

        self = self.run_command(&command);
        self
    }

    pub fn password(mut self, user: &User) -> Self {
        let username = user.username();
        let password = user.passwd();
        let password = &password.replace('$', r"\$").to_string();
        let command = format!(r#"echo "{username}:{password}" | chpasswd --encrypted"#);

        self = self.run_command(&command);
        self
    }


    pub fn chmod(mut self, permissions: u16, path: &str) -> Self {
        self.commands.push(
            format!(r#"--chmod '0{permissions}:{path}'"#)
        );
        self
    }

    pub fn append_line(mut self, path: &str, line: &str) -> Self {
        self.commands.push(
            format!(r#"--append-line '{path}:{line}'"#)
        );
        self
    }

    pub fn write(mut self, path: &str, content: &str) -> Self {
        self.commands.push(
            format!(r#"--write {path}:'{content}'"#)
        );
        self
    }

    pub fn build(self) -> Result<String, Box<dyn std::error::Error>> {
        let mut command = format!(r#"#!/bin/bash"#);
        command.push_str("\n");
        command.push_str(&format!(r#"virt-customize -a {IMAGE_PATH} \"#)); 
        for arg in self.commands {
            command.push_str("\n");
            command.push_str(&format!(r#"{arg} \"#));
        }
        Ok(command)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FormfileResponse {
    Success,
    Failure
}

pub fn routes() -> Router {
    Router::new()
        .route("/ping", post(handle_ping))
        .route("/:build_id/:instance_id/formfile", post(handle_formfile))
}

pub async fn serve_socket(_socket_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    todo!()
}

pub async fn serve(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("form-build-server attempting to bind to {}", addr);

    let router = routes();
    let listener = tokio::net::TcpListener::bind(
        addr
    ).await?;

    info!("form-build-server successfully bound and listening on {}", addr);

    if let Err(e) = axum::serve(listener, router).await {
        error!("Error in form-build-server: {}", e);
    }

    Ok(())
}

async fn handle_ping() -> Json<Value> {
    return Json(serde_json::json!({"ping": "pong"}));
}

async fn handle_formfile(
    AxumPath((build_id, instance_id)): AxumPath<(String, String)>,
    Json(formfile): Json<Formfile>,
) -> Json<FormfileResponse> {
    info!("Received /formfile request for build_id: {}, instance_id: {}", build_id, instance_id);
    info!("Parsed Formfile content: {:#?}", formfile);

    println!("Received formfile: {formfile:?}");
    let formfile = formfile;
    let workdir = formfile.workdir.clone().to_string_lossy().into_owned();
    info!("Target workdir for build: {}", workdir);
    println!("Request... Building command");
    let mut command = VirtCustomize::new()
        .run_command("growpart /dev/sda 1")
        .run_command("resize2fs /dev/sda1")
        .ssh_keygen()
        .mkdir(&workdir)
        .write("/etc/vm_name", &instance_id)
        .write("/etc/build_id", &build_id)
        .copy_in("/var/lib/formnet/formnet", "/usr/bin")
        .write("/etc/systemd/system/formnet-join.service", &write_formnet_join()) 
        .write("/etc/netplan/01-custom-netplan.yaml", &write_netplan())
        .run_command("apt-get -y update")
        .run_command("apt-get -y upgrade");

    info!("Base virt-customize commands added.");
    println!("Built base command...");

    // Create users
    for user in &formfile.users {
        info!("Processing user: {}", user.username());
        println!("Formfile contains users, adding users...");
        command = command.useradd(user);
        if !user.ssh_authorized_keys().is_empty() {
            info!("Injecting SSH keys for user: {}", user.username());
            command = command.ssh_inject(user);
        }
    }
    
    if formfile.users.is_empty() {
        info!("No users specified in Formfile.");
    }

    if no_copy(&formfile) {
        info!("Formfile contains COPY instructions, processing them individually.");
    } else {
        info!("No COPY instructions in Formfile, will attempt to copy entire /artifacts to {}", workdir);
        command = command.copy_in("/artifacts", &workdir);
    }

    for instruction in &formfile.build_instructions {
        info!("Processing build instruction: {:?}", instruction);
        println!("Discovered instruction: {instruction:?}...");
        match instruction {
            BuildInstruction::Install(opts) => { 
                info!("Adding install command for packages: {:?}", opts.packages);
                command = command.install(&opts.packages);
            },
            BuildInstruction::Run(cmd) => { 
                info!("Adding run command: {}", cmd);
                command = command.run_command(cmd); 
            } 
            BuildInstruction::Copy(from, to) => { 
                let from_str = from.to_string_lossy();
                let to_str = to.to_string_lossy();
                info!("Adding copy command from: {} to: {}", from_str, to_str);
                let from_abs = {
                    match from.as_path()
                        .strip_prefix("./")
                        .or_else(|_| {
                            from.as_path()
                                .strip_prefix("/")
                                .or_else(|_| {
                                    Ok::<&Path, Box<dyn std::error::Error>>(from.as_path())
                                })
                        }) {
                            Ok(f) => PathBuf::from("/artifacts").join(f.to_path_buf()).to_string_lossy().into_owned(),
                            Err(e) => {
                                error!("Error trying to convert COPY source path to absolute: {}. Original: {}", e, from_str);
                                return Json(FormfileResponse::Failure);
                            }
                        }
                };
                info!("Absolute COPY source: {}", from_abs);
                command = command.copy_in(&from_abs, &to_str); 
            },
            BuildInstruction::Entrypoint(entrypoint) => {
                info!("Processing ENTRYPOINT: command='{}', args='{:?}'", entrypoint.command(), entrypoint.args());
                let entrypoint_service_content = build_entrypoint(entrypoint);
                if !entrypoint_service_content.is_empty() {
                    info!("Writing systemd service for entrypoint: form-app.service");
                    command = command.write("/etc/systemd/system/form-app.service", &entrypoint_service_content);
                    command = command.chmod(644, "/etc/systemd/system/form-app.service");
                    command = command.run_command("systemctl enable form-app.service");
                } else {
                    info!("Entrypoint is empty, skipping systemd service creation.");
                }
            },
            BuildInstruction::Env(envvar) => { 
                info!("Adding ENV: {:?}", envvar);
                let (path, line) = add_env_var(envvar.clone()); 
                command = command.append_line(&path, &line);
            }
            BuildInstruction::Expose(_) => { 
                info!("Processing EXPOSE (currently a no-op in virt-customize stage)");
            } 
        }
        println!("added instruction: {instruction:?} to command...");
    }

    info!("Finalizing virt-customize commands with netplan and formnet enablement.");
    command = command.run_command("netplan apply");
    command = command.run_command("systemctl enable formnet-join.service");

    let final_virt_customize_script = match command.build() {
        Ok(cmd) => cmd,
        Err(e) => {
            error!("Error building final virt-customize script: {}", e);
            return Json(FormfileResponse::Failure)
        }
    };
    info!("Generated virt-customize script:\n{}", final_virt_customize_script);

    let mut file = match std::fs::File::create("/scripts/run-virt-customize.sh") {
        Ok(f) => f,
        Err(e) => {
            error!("Error creating virt-customize script file: {}", e);
            return Json(FormfileResponse::Failure);
        }
    };
    if let Err(e) = file.write_all(final_virt_customize_script.as_bytes()) {
        error!("Error writing to virt-customize script file: {}", e);
        return Json(FormfileResponse::Failure);
    }

    info!("Attempting to chmod +x /scripts/run-virt-customize.sh");
    let chmod_output = Command::new("chmod")
        .arg("+x")
        .arg("/scripts/run-virt-customize.sh")
        .output();

    match chmod_output {
        Ok(op) => {
            if !op.status.success() {
                let stderr = String::from_utf8_lossy(&op.stderr);
                error!("Failed to update script permissions for /scripts/run-virt-customize.sh. Stderr: {}", stderr);
                return Json(FormfileResponse::Failure);
            }
            info!("Successfully set +x on /scripts/run-virt-customize.sh");
        }
        Err(e) => {
            error!("Error attempting to chmod /scripts/run-virt-customize.sh: {}", e);
            return Json(FormfileResponse::Failure)
        }
    }

    info!("Attempting to run /scripts/run-virt-customize.sh");
    let output = Command::new("bash")
        .arg("/scripts/run-virt-customize.sh")
        .output();

    info!("virt-customize script execution finished.");

    match output {
        Ok(op) => {
            if !op.status.success() {
                let stderr = match std::str::from_utf8(&op.stderr) {
                    Ok(stderr) => stderr,
                    Err(e) => {
                        error!("Unable to capture command exit error: {}", e);
                        return Json(FormfileResponse::Failure)
                    }
                };
                error!("{stderr}");
                return Json(FormfileResponse::Failure);
            }

            let stdout = match std::str::from_utf8(&op.stdout) {
                Ok(stdout) => stdout,
                Err(e) => {
                    error!("Build successful, but could not capture stdout: {}", e);
                    return Json(FormfileResponse::Success)
                }
            };
            info!("{stdout}");
        }
        Err(e) => {
            error!("Output return error: {}", e);
            return Json(FormfileResponse::Failure)
        }
    }

    return Json(FormfileResponse::Success);
}

fn no_copy(formfile: &Formfile) -> bool {
    formfile.build_instructions.iter().any(|inst| matches!(inst, BuildInstruction::Copy(..)))
}

fn add_env_var(envvar: EnvVariable) -> (String, String) {
    let scope = envvar.scope;
    match scope {
        EnvScope::System => {
            return ("/etc/profile".to_string(), format!("{}={}", envvar.key, envvar.value));
        },
        EnvScope::User(user) => {
            return (format!("/home/{}/.bashrc", user), format!("{}={}", envvar.key, envvar.value));
        },
        EnvScope::Service(service) => {
            return (format!("/etc/{}.env", service), format!("{}={}", envvar.key, envvar.value));
        }, 
    }
}

fn build_entrypoint(
    entrypoint: &Entrypoint,
) -> String {
    let exec_start = if entrypoint.args().is_empty() {
        if entrypoint.command().is_empty() {
            return String::new()
        }
        entrypoint.command().to_string()
    } else {
        format!("{} {}", entrypoint.command(), entrypoint.args().join(" "))
    };

    return format!(r#"[Unit]
Description=Form Network Application Service
After=network.target       # Ensure network is available
Wants=network-online.target  # Prefer full network connectivity

[Service]
Type=simple               # Process directly runs the application
ExecStart={}             # Our application command
Restart=always           # Automatic restart on failure
RestartSec=3             # Wait 3 seconds before restart
EnvironmnetFile=/etc/form-app.env
StandardOutput=journal    # Log output to system journal
StandardError=journal     # Log errors to system journal
SyslogIdentifier=form-app

NoNewPrivileges=true     # Prevent privilege escalation
ProtectSystem=full       # Read-only access to system files
ProtectHome=true         # No access to home directories
PrivateTmp=true          # Private /tmp directory

[Install]
WantedBy=multi-user.target 
"#, exec_start);
}

pub fn copy_dir_recursively(
    source: impl AsRef<Path>,
    dest: impl AsRef<Path>
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&dest)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let filetype = entry.file_type()?;
        if filetype.is_dir() {
            copy_dir_recursively(entry.path(), dest.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dest.as_ref().join(entry.file_name()))?;
        }
    }

    Ok(())
}

fn write_netplan() -> String {
r#"network:
  version: 2
  renderer: networkd

  ethernets:
    rename-this-nic:
      match:
        name: "en*"
      set-name: eth0
      dhcp4: true
    "#.to_string()
}

fn write_formnet_join() -> String {
    format!(r#"[Unit]
Description=Formnet Join 
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
Environment="HOST_BRIDGE_IP={}"
ExecStart=/usr/bin/formnet instance 
StandardOutput=append:/var/log/formnet.log
StandardError=append:/var/log/formnet.log


[Install]
WantedBy=multi-user.target
"#, get_host_ip())
}

fn get_host_ip() -> String {
    std::env::var("HOST_BRIDGE_IP").unwrap()
}
