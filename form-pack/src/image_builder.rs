use std::path::Path;
use std::process::Command;
use axum::{routing::post, Json, Router};
use serde_json::Value;
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
            format!("--copy-in {from} {to}")
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
                format!(r#"--ssh-inject {username}:string:"{key}""#)
            );
        }
        self
    }
    
    pub fn useradd(mut self, user: &User) -> Self {
        let username = user.username();
        let passwd = user.passwd();
        let mut command = format!(r#"useradd {username} -m"#); 
        if user.sudo() && !user.disable_root() {
            command.push_str(" -g sudo");
        }

        if !user.groups().is_empty() {
            let groups = user.groups().join(",");
            command.push_str(&format!("-G {groups}"))
        }

        command.push_str(&format!("-p {passwd}"));

        if !user.chpasswd_expire() {
            command.push_str("-K PASS_MAX_DAYS=-1");
        }

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

    pub fn build(self) -> (String, Vec<String>) {
        let command = format!(r#"virt-customize -a {IMAGE_PATH}"#); 
        (command, self.commands)
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
    let mut command = VirtCustomize::new()
        // Grow the filesystem to match the disk size
        .run_command("growpart /dev/sda 1")
        .run_command("resize2fs /dev/sda1")
        // Create the workdir in the root directory of the disk 
        .mkdir(&workdir)
        // Update & Upgrade package manager
        .run_command("apt-get update")
        .run_command("apt-get upgrade");

    // Create users
    for user in &formfile.users {
        command = command.useradd(user);
    }
    
    // Check if there's any copy intructions
    // if not, recursively copy from 
    // /artifacts (formpack) to WORKDIR
    if no_copy(&formfile) {
        command = command.copy_in("/artifacts", &workdir)
    }

    for instruction in &formfile.build_instructions {
        match instruction {
            BuildInstruction::Install(opts) => { command = command.install(&opts.packages); },
            BuildInstruction::Run(cmd) => { command = command.run_command(cmd); } 
            BuildInstruction::Copy(from, to) => { 
                let from = from.to_string_lossy().into_owned();
                let to = to.to_string_lossy().into_owned();
                command = command.copy_in(&from, &to) 
            },
            BuildInstruction::Entrypoint(entrypoint) => {
                let entrypoint = build_entrypoint(entrypoint);
                if !entrypoint.is_empty() {
                    command = command.write("/etc/systemd/system/form-app.service", &entrypoint);
                    command = command.chmod(644, "/etc/systemd/form-app.service");
                    command = command.run_command("systemctl enable form-app.service");
                }
            },
            BuildInstruction::Env(envvar) => {
                let (path, line) = add_env_var(envvar.clone()); 
                command = command.append_line(&path, &line)
            }
            BuildInstruction::Expose(_) => {} 
        }
    }

    let (command, args) = command.build();

    try_failure!(
        Command::new(command)
            .args(&args)
            .status()
    );

    return Json(FormfileResponse::Success);
}

fn no_copy(formfile: &Formfile) -> bool {
    formfile.build_instructions.iter().any(|inst| {
        match inst {
            BuildInstruction::Copy(..) => true,
            _ => false,
        }}
    )
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

# Security hardening options
NoNewPrivileges=true     # Prevent privilege escalation
ProtectSystem=full       # Read-only access to system files
ProtectHome=true         # No access to home directories
PrivateTmp=true          # Private /tmp directory

[Install]
WantedBy=multi-user.target  # Start on system boot
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
