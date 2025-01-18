use std::path::{Path, PathBuf};
use std::process::Command;
use axum::{routing::post, Json, Router};
use serde_json::Value;
use std::io::Write;
use serde::{Serialize, Deserialize};
use crate::formfile::{BuildInstruction, Entrypoint, EnvScope, EnvVariable, Formfile, User};

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
        let packages: String = packages.join(" ");
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
        let mut command = format!(r#"useradd -m -s /bin/bash {username}"#); 
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

macro_rules! try_failure {
    ($expr:expr) => {
        match $expr {
            Ok(value) => value,
            Err(_) => return Json(FormfileResponse::Failure),
        }
    };
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FormfileResponse {
    Success,
    Failure
}

pub fn routes() -> Router {
    Router::new()
        .route("/ping", post(handle_ping))
        .route("/formfile", post(handle_formfile))
}

pub async fn serve_socket(_socket_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    todo!()
}

pub async fn serve(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Attempting to bind to {addr}");

    let router = routes();
    let listener = tokio::net::TcpListener::bind(
        addr
    ).await?;

    if let Err(e) = axum::serve(listener, router).await {
        eprintln!("Error in FormPackManager API Server: {e}");
    }

    Ok(())
}

async fn handle_ping() -> Json<Value> {
    return Json(serde_json::json!({"ping": "pong"}));
}

async fn handle_formfile(
    Json(formfile): Json<Formfile>,
) -> Json<FormfileResponse> {
    let formfile = formfile;
    let workdir = formfile.workdir.clone().to_string_lossy().into_owned();
    println!("Request... Building command");
    let mut command = VirtCustomize::new()
        //TODO: if a `DISK` option is provided in the formfile,
        //increase the disk by running an exec to the container
        //calling qemu-img resize
        // Grow the filesystem to match the disk size
        .run_command("growpart /dev/sda 1")
        .run_command("resize2fs /dev/sda1")
        // Create the workdir in the root directory of the disk 
        .mkdir(&workdir)
        // Update & Upgrade package manager
        .copy_in("/var/lib/formnet/formnet", "/usr/bin")
        .write("/etc/systemd/system/formnet-up.service", &write_formnet_up()) 
        .write("/etc/systemd/system/formnet-install.service", &write_formnet_install()) 
        .write("/etc/netplan/01-custom-netplan.yaml", &write_netplan())
        .run_command("apt-get -y update")
        .run_command("apt-get -y upgrade");
    println!("Built base command...");

    // Create users
    for user in &formfile.users {
        println!("Formfile containers users, adding users...");
        command = command.useradd(user);
        command = command.password(user);
    }
    
    // Check if there's any copy intructions
    // if not, recursively copy from 
    // /artifacts (formpack) to WORKDIR
    if no_copy(&formfile) {
        println!("No Copy instructions found, copying entire artifacts folder...");
        command = command.copy_in("/artifacts", &workdir)
    }

    for instruction in &formfile.build_instructions {
        println!("Discovered instruction: {instruction:?}...");
        match instruction {
            BuildInstruction::Install(opts) => { command = command.install(&opts.packages); },
            BuildInstruction::Run(cmd) => { command = command.run_command(cmd); } 
            BuildInstruction::Copy(from, to) => { 
                let from = {
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
                                println!("Error trying to convert to absolute path: {e}");
                                return Json(FormfileResponse::Failure);
                            }
                        }
                };

                let to = to.to_string_lossy().into_owned();
                command = command.copy_in(&from, &to) 
            },
            BuildInstruction::Entrypoint(entrypoint) => {
                let entrypoint = build_entrypoint(entrypoint);
                if !entrypoint.is_empty() {
                    command = command.write("/etc/systemd/system/form-app.service", &entrypoint);
                    command = command.chmod(644, "/etc/systemd/system/form-app.service");
                    command = command.run_command("systemctl enable form-app.service");
                }
            },
            BuildInstruction::Env(envvar) => {
                let (path, line) = add_env_var(envvar.clone()); 
                command = command.append_line(&path, &line)
            }
            BuildInstruction::Expose(_) => {} 
        }

        println!("added instruction: {instruction:?} to command...");
    }

    println!("Adding netplan apply and formnet commands to command...");
    command = command.run_command("netplan apply");
    command = command.run_command("systemctl enable formnet-up.service");
    command = command.run_command("systemctl enable formnet-install.service");

    let command = match command.build() {
        Ok(cmd) => cmd,
        Err(e) => {
            println!("Error building command {e}");
            return Json(FormfileResponse::Failure)
        }
    };
    println!("Built command and args, running...");
    println!("{command}");

    let mut file = match std::fs::File::create("/scripts/run-virt-customize.sh") {
        Ok(f) => f,
        Err(e) => {
            println!("Error creating script file: {e}");
            return Json(FormfileResponse::Failure);
        }
    };
    let chmod_output = Command::new("chmod")
        .arg("+x")
        .arg("/scripts/run-virt-customize.sh")
        .output();

    let _ = file.write_all(command.as_bytes());

    match chmod_output {
        Ok(op) => {
            if !op.status.success() {
                println!("Failed to update script permissions");
                return Json(FormfileResponse::Failure);
            }
        }
        Err(e) => {
            println!("Error attempting to chmod script permissions: {e}");
            return Json(FormfileResponse::Failure)
        }
    }

    let output = Command::new("bash")
        .arg("/scripts/run-virt-customize.sh")
        .arg(&command)
        .output();

    match output {
        Ok(op) => {
            if !op.status.success() {
                let stderr = match std::str::from_utf8(&op.stderr) {
                    Ok(stderr) => stderr,
                    Err(e) => {
                        println!("Unable to capture command exit error: {e}");
                        return Json(FormfileResponse::Failure)
                    }
                };
                println!("{stderr}");
                return Json(FormfileResponse::Failure);
            }

            let stdout = match std::str::from_utf8(&op.stdout) {
                Ok(stdout) => stdout,
                Err(e) => {
                    println!("Build successful, but could not capture stdout: {e}");
                    return Json(FormfileResponse::Success)
                }
            };
            println!("{stdout}");
        }
        Err(e) => {
            println!("Output return error: {e}");
            return Json(FormfileResponse::Failure)
        }
    }

    println!("Successfully executed command...");

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

fn write_formnet_up() -> String {
r#"[Unit]
Description=Formnet Up
After=formnet-install.service
Wants=formnet-install.service
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/formnet up -d --interval 60
Restart=always
RestartSec=5
StandardOutput=append:/var/log/formnet.log
StandardError=append:/var/log/formnet.log


[Install]
WantedBy=multi-user.target
"#.to_string()
}

fn write_formnet_install() -> String {
r#"[Unit]
Description=Formnet Install
After=network-online.target
Wants=network-online.target

ConditionPathExists=!/etc/formnet/state.toml

[Service]
Type=oneshot
ExecStart=/usr/local/bin/formnet install --default-name -d /etc/formnet/invite.toml
ExecStart=/bin/touch /etc/formnet/state.toml
RemainAfterExit=yes
StandardOutput=append:/var/log/formnet.log
StandardError=append:/var/log/formnet.log

[Install]
WantedBy=multi-user.target
"#.to_string()
}
