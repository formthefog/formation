use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sha_crypt::{sha512_crypt_b64, Sha512Params};
use serde::{Serialize, Deserialize};
use std::{collections::{HashMap, HashSet}, path::{Component, PathBuf}};

pub struct FormfileParser {
    current_line: usize,
    instructions: Vec<BuildInstruction>,
    system_config: Vec<SystemConfigOpt>,
    users: Vec<User>,
    workdir: Option<PathBuf>,
    entrypoint: Entrypoint,
}

impl FormfileParser {
    pub fn new() -> Self {
        Self {
            current_line: 0,
            instructions: Vec::new(),
            system_config: Vec::new(),
            users: Vec::new(),
            workdir: None,
            entrypoint: Entrypoint {
                command: String::new(),
                args: Vec::new()
            }
        }
    }

    pub fn parse(&mut self, content: &str) -> Result<Formfile, Box<dyn std::error::Error>> {
        for line in content.lines() {
            self.current_line += 1;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            self.parse_line(line)?;
        }

        self.build_formfile()
    }

    pub fn parse_line(&mut self, line: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut parts = line.splitn(2, ' ');
        let instruction = parts.next()
            .ok_or(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "empty line encountered"
                    )
                )
            )?;

        let args = parts.next()
            .ok_or(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "Line has instruction but no arg: {}",
                            self.current_line
                        )
                    )
                )
            )?;

        match instruction {
            "RUN" => {
                self.instructions.push(
                    BuildInstruction::Run(args.to_string())
                );
            },
            "COPY" => {
                let path = PathBuf::from(args);
                self.instructions.push(
                    BuildInstruction::Copy(path)
                );
            },
            "INSTALL" => self.parse_install(args)?,
            "ENV" => self.parse_env(args)?,
            "USER" => self.parse_user(args)?,
            "VCPU" | "CPU" | "CORES" => self.parse_vcpus(args)?,
            "MEMORY" | "MEM" | "MBS" => self.parse_memory(args)?,
            "DISK" | "STORAGE" => self.parse_disk(args)?,
            "WORKDIR" => self.parse_workdir(args)?,
            "ENTRYPOINT" => self.parse_entrypoint(args)?,
            _ => {}
        }


        Ok(())
    }

    fn split_command_string(&self, input: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    in_quotes = !in_quotes;
                }
                ' ' => {
                    if !current.is_empty() {
                        result.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if in_quotes {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Unclosed quotes in ENTRYPOINT on line {}", self.current_line)
                    )
                )
            );
        }

        if !current.is_empty() {
            result.push(current);
        }

        Ok(result)
    }

    pub fn parse_entrypoint(
        &mut self,
        args: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let trimmed = args.trim();

        if trimmed.is_empty() {
            return Ok(())
        }
        
        // Handle JSON array format: ["command", "arg1", "arg2"]
        if trimmed.starts_with('[') {
            if !trimmed.ends_with(']') {
                return Err(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!(
                                "Malformed JSON array in ENTRYPOIONT - missing closing bracket on line {}",
                                self.current_line
                            )
                        )
                    )
                );
            }

            let mut parts: Vec<String> = serde_json::from_str(&trimmed).map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Invalid JSON in ENTRYPOINT array on line {}: {}", self.current_line, e)
                ))
            })?;
            
            if parts.is_empty() {
                return Ok(())
            }

            let command = parts.remove(0).clone();
            let args = parts;

            self.instructions.push(BuildInstruction::Entrypoint(Entrypoint {
                command,
                args,
            }));
        } else {
            let mut parts = self.split_command_string(trimmed)?; 
            if parts.is_empty() {
                return Ok(())
            }

            let command = parts.remove(0).clone();
            let args = parts.clone();

            self.instructions.push(BuildInstruction::Entrypoint(Entrypoint {
                command,
                args,
            }));
        }

        Ok(())
    }

    pub fn parse_workdir(
        &mut self,
        path: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.trim();

        if !path.starts_with('/') {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "WORKDIR must be an absolute path: line {}: {}",
                            self.current_line,
                            path
                        )
                    )
                )
            );
        }

        let path_buf = PathBuf::from(path);
        if path_buf.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "WORKDIR cannot contain . or .. on line {}: {}",
                            self.current_line,
                            path
                        )
                    )
                )
            );
        }
        
        if self.workdir.is_some() {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "WORKDIR already declared, second one found on line {}",
                            self.current_line
                        )
                    )
                )
            );
        }

        self.workdir = Some(path_buf);

        Ok(())
    }

    pub fn parse_vcpus(&mut self, vcpu_count: &str) -> Result<(), Box<dyn std::error::Error>> {
        if vcpu_count.is_empty() {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("If VCPU | CPU | CORES key is provided, a value must be provided as well: line {}", self.current_line)
                    )
                )
            );
        }

        let vcpu_count: u8 = vcpu_count.parse()?;

        if vcpu_count <= 0 || vcpu_count > 128 {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Value provided for VCPU | CPU | CORES is invalid, must be at least 1 and no greater than 128: line {}: {}",
                            self.current_line,
                            vcpu_count
                        )
                    )
                )
            );
        }

        self.system_config.push(
            SystemConfigOpt::Cpu(vcpu_count)
        );

        Ok(())
    }

    pub fn parse_memory(&mut self, mem_alloc: &str) -> Result<(), Box<dyn std::error::Error>> {
        if mem_alloc.is_empty() {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("If MEMORY | MEM | MBS key is provided, a value must be provided as well: line: {}", self.current_line) 
                    )
                )
            );
        }

        let memory: usize = mem_alloc.parse()?;

        if memory < 512 || memory > 256000 {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Invalid value provided for MEMORY | MEM | MBS. Value must bet between at least 512 and at most 256000: line: {} {}",
                            self.current_line,
                            mem_alloc
                        )
                    )
                )
            );
        }

        self.system_config.push(
            SystemConfigOpt::Memory(memory)
        );

        Ok(())
    }

    pub fn parse_disk(&mut self, storage_size: &str) -> Result<(), Box<dyn std::error::Error>> {
        if storage_size.is_empty() {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("If DISK | STORAGE key is provided a value must be provided as well: line: {}", self.current_line) 
                    )
                )
            );
        }

        let disk_size: u16 = storage_size.parse()?;

        if disk_size < 5 || disk_size > u16::MAX {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Invalid value provided for DISK | STORAGE. Value must bet at least 5 and at most {}: line {}: {}",
                            u16::MAX,
                            self.current_line,
                            disk_size
                        )
                    )
                )
            );
        }


        self.system_config.push(
            SystemConfigOpt::Disk(disk_size)
        );

        Ok(())
    }

    pub fn parse_user(&mut self, args: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut user = User {
            username: String::new(),
            passwd: String::new(),
            ssh_authorized_keys: Vec::new(),
            lock_passwd: false,
            sudo: false,
            shell: "/bin/bash".to_string(),
            ssh_pwauth: true,
            disable_root: true,
            chpasswd_expire: true,
            chpasswd_list: HashMap::new(),
            groups: Vec::new(),
        };

        let parts = self.split_preserving_quotes(args)?;

        for part in parts {
            let (field, value) = if part.contains(':') {
                let mut split = part.splitn(2, ':');
                (split.next().unwrap(), split.next().unwrap_or(""))
            } else if part.contains('=') {
                let mut split = part.splitn(2, '=');
                (split.next().unwrap(), split.next().unwrap_or(""))
            } else {
                return Err(Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "Invalid USER field format on line {}. Expected field:value or field=value, got: {}",
                            self.current_line,
                            part
                        )
                    )
                ));
            };

            let value = value.trim_matches('"').trim_matches('\'');

            match field.trim() {
                "username" => {
                    self.validate_username(value)?;
                    user.username = value.to_string();
                }
                "passwd" => {
                    if value.is_empty() {
                        if value.is_empty() {
                            return Err(Box::new(
                                std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    format!("Password cannot be empty on line {}", self.current_line)
                                )
                            ));
                        }
                    }
                    user.passwd = self.hash_password(value, None)?;
                }
                "ssh_authorized_keys" => {
                    user.ssh_authorized_keys = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect();
                }
                "lock_passwd" => {
                    user.lock_passwd = self.parse_bool_value(value)?;
                }
                "sudo" => {
                    user.sudo = self.parse_bool_value(value)?;
                }
                "shell" => {
                    if !value.starts_with('/') {
                        return Err(Box::new(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!("Shell path must be absolute on line {}: {}",
                                    self.current_line,
                                    value
                                )
                            )
                        ));
                    }
                }
                "ssh_pwauth" => {
                    user.ssh_pwauth = self.parse_bool_value(value)?;
                }
                "disable_root" => {
                    user.disable_root = self.parse_bool_value(value)?;
                }
                "chpasswd_expire" => {
                    user.chpasswd_expire = self.parse_bool_value(value)?;
                }
                "groups" => {
                    user.groups = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect();
                }
                _ => return Err(Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "Unknown USER field on line {}: {}",
                            self.current_line,
                            field
                        )
                    )
                ))
            }
        };

        self.users.push(user);

        Ok(())
    }

    pub fn gen_salt(&self, length: usize) -> String {
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(length)
            .map(char::from)
            .collect()
    }

    pub fn hash_password(&self, password: &str, salt: Option<&str>) -> Result<String, Box<dyn std::error::Error>>{
        let params = Sha512Params::new(4096).map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{e:?}")))
        })?;

        let salt = if let Some(salt_str) = salt {
            salt_str.to_string()
        } else {
            self.gen_salt(16)
        };

        let hashed = sha512_crypt_b64(password.as_bytes(), &salt.as_bytes(), &params).map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{e:?}")))
        })?; 

        let hashed = format!("$6$rounds=4096${salt}${hashed}");

        Ok(hashed)
    }

    pub fn split_preserving_quotes(
        &self,
        value: &str
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_char = '"';

        for c in value.chars() {
            match c {
                '"' | '\'' => {
                    if !in_quotes {
                        in_quotes = true;
                        quote_char = c;
                    } else if c == quote_char {
                        in_quotes = false;
                    } else {
                        current.push(c);
                    }
                }
                ' ' if !in_quotes => {
                    if !current.is_empty() {
                        result.push(current.clone());
                        current.clear();
                    }
                }
                _ => current.push(c),
            }
        }

        if in_quotes {
            return Err(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Unclosed quotes on line {}", self.current_line),
                )
            ));
        }

        if !current.is_empty() {
            result.push(current);
        }

        Ok(result)
    }

    pub fn validate_username(
        &self,
        username: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        if username.is_empty() {
            return Err(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Username cannot be empty")
                )
            ));
        }

        let first_char = username.chars().next().unwrap();

        if !first_char.is_ascii_lowercase() && first_char != '_' {
            return Err(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Username must start with a lowercase letter or underscore on line {}: {}",
                        self.current_line,
                        username
                    )
                )
            ));
        }

        if !username.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'
        }) {
            return Err(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Username can only contain lowercase letters, numbers, underscores or hyphens on line {}: {}",
                        self.current_line,
                        username
                    )
                )
            ));
        }

        if username.len() > 32 {
            return Err(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Username cannot be longer than 32 characters on line {}: {}",
                        self.current_line,
                        username
                    )
                )
            ));
        }

        Ok(())
    }

    pub fn parse_bool_value(
        &self,
        value: &str
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match value.to_lowercase().as_str() {
            "true" | "yes" | "1" | "on" => Ok(true),
            "false" | "no" | "0" | "off" => Ok(false),
            _ => return Err(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Invalid boolean value on line {}: {}. Expected true/false, yes/no, 1/0 or on/off",
                        self.current_line,
                        value
                    )
                )
            )),
        }
    }

    pub fn parse_install(&mut self, args: &str) -> Result<(), Box<dyn std::error::Error>> { 
        let packages: Vec<String> = args.split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if packages.is_empty() {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("If INSTALL key is provided at least 1 pacage to install must be provided as well: line {}", self.current_line)
                    )
                )
            );
        }

        let opts = InstallOpts { packages };

        self.instructions.push(
            BuildInstruction::Install(opts)
        );

        Ok(())
    }

    pub fn parse_env(&mut self, args: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut parts: Vec<&str> = args.split_whitespace().collect(); 

        let scope = if parts[0].starts_with("--scope=") {
            let scope = self.parse_env_scope(parts[0])?;
            parts.remove(0);
            scope
        } else {
            EnvScope::System
        };

        if parts.len() != 1 {
            return Err(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Invalid ENV format on line {}. Expected: ENV [--scope=<scope>] KEY=value",
                        self.current_line
                    )
                )
            ));
        }

        let (key, value) = self.parse_env_pair(parts[0])?;

        let env_var = EnvVariable {
            key,
            value,
            scope
        };

        self.instructions.push(BuildInstruction::Env(env_var));
        Ok(())
    }

    pub fn parse_env_scope(
        &mut self,
        args: &str
    ) -> Result<EnvScope, Box<dyn std::error::Error>> {
        // The scope argument should start with --scope=
        // We'll use strip_prefix to remove it and get the actual scope value
        let scope_value = args 
            .strip_prefix("--scope=")
            .ok_or(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "Invalid scope format on line {}. Expected --scope=<scope>, got: {}",
                            self.current_line,
                            args
                        )
                    )
                )
            )?;

        match scope_value {
            "system" => Ok(EnvScope::System),
            s if s.starts_with("user:") => {
                let username = s.strip_prefix("user:").unwrap().to_string();

                if username.is_empty() {
                    return Err(
                        Box::new(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!(
                                    "Empty username in scope on line {}. Format should be --scope=user:<username>",
                                    self.current_line
                                )
                            )
                        )
                    );
                }

                Ok(EnvScope::User(username))
            }
            s if s.starts_with("service:") => {
                let service = s.strip_prefix("service:")
                    .unwrap();

                if service.is_empty() {
                    return Err(
                        Box::new(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!(
                                    "Empty service name in scope on line {}. Format should be --scope=service:<service>",
                                    self.current_line
                                )
                            )
                        )
                    )
                }

                Ok(EnvScope::Service(service.to_string()))
            }
            _ => return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "Invalid scope on line {}. Expected 'system', 'user:<name>' or 'service:<service>', got: {}",
                            self.current_line,
                            scope_value
                        )
                    )
                )
            )
        }
    }

    pub fn parse_env_pair(
        &mut self,
        pair: &str
    ) -> Result<(String, String), Box<dyn std::error::Error>> {
        let parts: Vec<&str> = pair.splitn(2, '=').collect();

        if parts.len() !=2 {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "Invalid environment variable format on line {}. Expected KEY=value: {}",
                            self.current_line,
                            pair
                        )
                    )
                )
            );
        }

        let key = parts[0].trim();
        let value = parts[1].trim();

        if key.is_empty() {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Empty environment variable name on line {}", self.current_line)
                    )
                )
            );
        }

        // Environment variable names should follow standard Unix conventions
        // - start with a letter or underscore
        // - Contains only letters, numbers, and underscores
        let first_char = key.chars().next().ok_or(
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Invalid environment variable name on line {}: Names must not be empty", self.current_line)
                )
            )
        )?;

        if !first_char.is_ascii_alphabetic() && first_char != '_' {
            return Err(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Invalid environment variable name on line {}: {}. Names must start with a letter or underscore",
                        self.current_line,
                        key
                    )
                )
            ));
        }

        if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "Invalid environment variable name on line {}: {}. Names can only contain letters, numbers and underscores",
                            self.current_line,
                            key
                        )
                    )
                )
            );
        }

        Ok((key.to_string(), value.to_string()))
    }

    pub fn build_formfile(&self) -> Result<Formfile, Box<dyn std::error::Error>> {
        Ok(Formfile {
            build_instructions: self.instructions.clone(),
            system_config: self.system_config.clone(),
            users: self.users.clone(),
            workdir: self.workdir.clone().unwrap_or(PathBuf::from("/app"))
        })
    }
}


/// Represents a complete parsed Formfile with all of its instructions
/// and configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formfile {
    ///  Build time instructions that modify the image
    pub build_instructions: Vec<BuildInstruction>,
    /// System configuration for the VM
    pub system_config: Vec<SystemConfigOpt>,
    /// User configurations
    pub users: Vec<User>,
    /// Working directory for the application
    pub workdir: PathBuf
}

/// Instructions that are executed during teh image build phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildInstruction {
    /// Run a command in the image as root
    Run(String),
    /// Copy files from the build context to a temporary artifacts
    /// directory that will be tarballed.
    /// if none is provided a default . will be added, and ALL files
    /// from the directory will be copied into the artifacts directory,
    /// tarballed and then copied into the WORKDIR
    Copy(PathBuf),
    /// Install system packages, this can be done with Run command as well,
    /// however, this particular command ONLY installs packages using apt-get
    Install(InstallOpts),
    /// Environment variables in Formfile are different from Dockerfile, in
    /// that they require a Scope. Scope can be system (system wide env vars)
    /// user specific, or service specific
    Env(EnvVariable),
    Entrypoint(Entrypoint),
    Expose(HashSet<u16>)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallOpts {
    packages: Vec<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemConfigOpt {
    Cpu(u8),
    Memory(usize),
    Disk(u16),
    // Devices (GPUs, etc.)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    username: String,
    passwd: String,
    ssh_authorized_keys: Vec<String>,
    lock_passwd: bool,
    sudo: bool,
    shell: String,
    ssh_pwauth: bool,
    disable_root: bool,
    chpasswd_expire: bool,
    chpasswd_list: HashMap<String, String>,
    groups: Vec<String>
}

pub struct UserBuilder {
    username: Option<String>,
    passwd: Option<String>,
    ssh_authorized_keys: Option<Vec<String>>,
    lock_passwd: Option<bool>,
    sudo: Option<bool>,
    shell: Option<String>,
    ssh_pwauth: Option<bool>,
    disable_root: Option<bool>,
    chpasswd_expire: Option<bool>,
    chpasswd_list: Option<HashMap<String, String>>,
    groups: Option<Vec<String>>
}

impl Default for UserBuilder {
    fn default() -> Self {
        Self {
            username: None, 
            passwd: None,
            ssh_authorized_keys: None,
            lock_passwd: None,
            sudo: None,
            shell: None,
            ssh_pwauth: None, 
            disable_root: None,
            chpasswd_expire: None,
            chpasswd_list: None,
            groups: None, 
        }
    }
}

impl UserBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn username(mut self, un: &str) -> Self {
       self.username = Some(un.to_string());
       self
    } 
    pub fn passwd(mut self, pw: &str) -> Self {
        self.passwd = Some(pw.to_string());
        self
    }
    pub fn ssh_authorized_keys(mut self, ak: Vec<String>) -> Self {
        self.ssh_authorized_keys = Some(ak);
        self
    }

    pub fn lock_passwd(mut self, lp: bool) -> Self {
        self.lock_passwd = Some(lp);
        self
    }

    pub fn sudo(mut self, sudo: bool) -> Self {
        self.sudo = Some(sudo);
        self
    }

    pub fn shell(mut self, shell: &str) -> Self {
        self.shell = Some(shell.to_string());
        self
    }

    pub fn ssh_pwauth(mut self, ssh_pwauth: bool) -> Self {
        self.ssh_pwauth = Some(ssh_pwauth);
        self
    } 

    pub fn disable_root(mut self, disable_root: bool) -> Self {
        self.disable_root = Some(disable_root);
        self
    }

    pub fn chpasswd_expire(mut self, chpw_expire: bool) -> Self {
        self.chpasswd_expire = Some(chpw_expire);
        self
    }

    pub fn chpasswd_list(mut self, chpw_list: HashMap<String, String>) -> Self {
        self.chpasswd_list = Some(chpw_list);
        self
    }

    pub fn groups(mut self, groups: Vec<String>) -> Self {
        self.groups = Some(groups);
        self
    }

    pub fn build(self) -> Result<User, String> {
        Ok(User {
            username: self.username.ok_or("Username is required".to_string())?,
            passwd: self.passwd.ok_or("Password is required".to_string())?,
            ssh_authorized_keys: self.ssh_authorized_keys.unwrap_or(Vec::new()),
            lock_passwd: self.lock_passwd.unwrap_or(false),
            sudo: self.sudo.unwrap_or(false),
            shell: self.shell.unwrap_or("/bin/bash".to_string()),
            ssh_pwauth: self.ssh_pwauth.unwrap_or(false),
            disable_root: self.disable_root.unwrap_or(true),
            chpasswd_expire: self.chpasswd_expire.unwrap_or(true),
            chpasswd_list: self.chpasswd_list.unwrap_or(HashMap::new()),
            groups: self.groups.unwrap_or(Vec::new())
        })
    }
}

impl User {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn passwd(&self) -> &str {
        &self.passwd
    }

    pub fn ssh_authorized_keys(&self) -> &[String] {
        &self.ssh_authorized_keys
    }

    pub fn lock_passwd(&self) -> bool {
        self.lock_passwd
    }

    pub fn set_lock_passwd(&mut self, lock_passwd: bool) {
        self.lock_passwd = lock_passwd;
    }

    pub fn sudo(&self) -> bool {
        self.sudo
    }

    pub fn set_sudo(&mut self, sudo: bool) {
        self.sudo = sudo
    }

    pub fn shell(&self) -> &str {
        &self.shell
    }

    pub fn ssh_pwauth(&self) -> bool {
        self.ssh_pwauth
    }

    pub fn disable_root(&self) -> bool {
        self.disable_root
    }

    pub fn chpasswd_expire(&self) -> bool {
        self.chpasswd_expire
    }

    pub fn chpasswd_list(&self) -> &HashMap<String, String> {
        &self.chpasswd_list
    }

    pub fn groups(&self) -> &[String] {
        &self.groups
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVariable {
    pub key: String,
    pub value: String,
    pub scope: EnvScope
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnvScope {
    System,
    User(String),
    Service(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entrypoint {
    command: String,
    args: Vec<String>,
    // Add log options
    // Add env options
}

pub struct EntrypointBuilder {
    pub command: Option<String>,
    pub args: Option<Vec<String>> 
}

impl Default for EntrypointBuilder {
    fn default() -> Self {
        Self { command: None, args: None }
    }
}

impl EntrypointBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn command(mut self, command: &str) -> Self {
        self.command = Some(command.to_string());
        self
    }

    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = Some(args);
        self
    }

    pub fn build(self) -> Entrypoint {
        Entrypoint {
            command: self.command.unwrap_or(String::new()),
            args: self.args.unwrap_or(Vec::new())
        }
    }
}

impl Entrypoint {
    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test the basic parser initialization
    #[test]
    fn test_parser_initialization() {
        let parser = FormfileParser::new();
        assert!(parser.instructions.is_empty());
        assert!(parser.system_config.is_empty());
        assert!(parser.users.is_empty());
        assert!(parser.workdir.is_none());
        assert_eq!(parser.current_line, 0);
    }

    // Test empty and comment line handling
    #[test]
    fn test_empty_and_comment_lines() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = FormfileParser::new();
        let content = r#"
        # This is a comment
        
        # Another comment
        "#;
        let result = parser.parse(content)?;
        assert!(result.build_instructions.is_empty());
        assert!(result.system_config.is_empty());
        assert!(result.users.is_empty());
        Ok(())
    }

    // Test system configuration parsing
    #[test]
    fn test_vcpu_parsing() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = FormfileParser::new();
        
        // Test valid CPU configurations
        parser.parse_vcpus("4")?;
        assert!(matches!(parser.system_config[0], SystemConfigOpt::Cpu(4)));

        // Test invalid configurations
        assert!(parser.parse_vcpus("0").is_err()); // Too low
        assert!(parser.parse_vcpus("129").is_err()); // Too high
        assert!(parser.parse_vcpus("-1").is_err()); // Negative
        assert!(parser.parse_vcpus("abc").is_err()); // Non-numeric

        Ok(())
    }

    #[test]
    fn test_memory_parsing() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = FormfileParser::new();
        
        // Test valid memory configurations
        parser.parse_memory("1024")?;
        assert!(matches!(parser.system_config[0], SystemConfigOpt::Memory(1024)));

        // Test invalid configurations
        assert!(parser.parse_memory("256").is_err()); // Too low
        assert!(parser.parse_memory("300000").is_err()); // Too high
        assert!(parser.parse_memory("-1024").is_err()); // Negative
        assert!(parser.parse_memory("abc").is_err()); // Non-numeric

        Ok(())
    }

    #[test]
    fn test_disk_parsing() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = FormfileParser::new();
        
        // Test valid disk configurations
        parser.parse_disk("20")?;
        assert!(matches!(parser.system_config[0], SystemConfigOpt::Disk(20)));

        // Test invalid configurations
        assert!(parser.parse_disk("2").is_err()); // Too low
        assert!(parser.parse_disk("-10").is_err()); // Negative
        assert!(parser.parse_disk("abc").is_err()); // Non-numeric

        Ok(())
    }

    // Test environment variable parsing
    #[test]
    fn test_env_parsing() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = FormfileParser::new();

        // Test system scope (default)
        parser.parse_env("PATH=/usr/local/bin")?;
        
        // Test user scope
        parser.parse_env("--scope=user:alice HOME=/home/alice")?;
        
        // Test service scope
        parser.parse_env("--scope=service:webapp PORT=8080")?;

        // Verify the parsed instructions
        let env_vars: Vec<&BuildInstruction> = parser.instructions.iter()
            .filter(|i| matches!(i, BuildInstruction::Env(_)))
            .collect();
        
        assert_eq!(env_vars.len(), 3);

        // Test invalid formats
        assert!(parser.parse_env("INVALID_FORMAT").is_err());
        assert!(parser.parse_env("--scope=invalid KEY=value").is_err());
        assert!(parser.parse_env("--scope=user: KEY=value").is_err());
        assert!(parser.parse_env("1INVALID=value").is_err());

        Ok(())
    }

    // Test user configuration parsing
    #[test]
    fn test_user_parsing() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = FormfileParser::new();

        // Test complete user configuration
        parser.parse_user(r#"username:testuser passwd:testpass123 sudo:yes shell:/bin/bash groups:docker,users"#)?;
        
        let user = &parser.users[0];
        assert_eq!(user.username, "testuser");
        assert!(!&user.passwd.is_empty());
        assert!(&user.passwd.starts_with("$6$rounds=4096$"));
        assert!(user.sudo);
        assert_eq!(user.shell, "/bin/bash");
        assert_eq!(user.groups, vec!["docker", "users"]);

        // Test invalid configurations
        assert!(parser.parse_user("username:").is_err()); // Empty username
        assert!(parser.parse_user("username:INVALID!").is_err()); // Invalid username chars
        assert!(parser.parse_user("username:test passwd:").is_err()); // Empty password
        assert!(parser.parse_user("username:test shell:invalid").is_err()); // Invalid shell path

        Ok(())
    }

    // Test installation parsing
    #[test]
    fn test_install_parsing() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = FormfileParser::new();

        // Test basic package installation
        parser.parse_install("nginx python3 postgresql")?;
        
        if let BuildInstruction::Install(opts) = &parser.instructions[0] {
            assert_eq!(opts.packages, vec!["nginx", "python3", "postgresql"]);
        } else {
            panic!("Expected Install instruction");
        }

        // Test empty package list
        assert!(parser.parse_install("").is_err());

        Ok(())
    }

    // Test boolean value parsing
    #[test]
    fn test_bool_value_parsing() -> Result<(), Box<dyn std::error::Error>> {
        let parser = FormfileParser::new();

        // Test valid boolean values
        assert!(parser.parse_bool_value("true")?);
        assert!(parser.parse_bool_value("yes")?);
        assert!(parser.parse_bool_value("1")?);
        assert!(parser.parse_bool_value("on")?);
        assert!(!parser.parse_bool_value("false")?);
        assert!(!parser.parse_bool_value("no")?);
        assert!(!parser.parse_bool_value("0")?);
        assert!(!parser.parse_bool_value("off")?);

        // Test invalid boolean values
        assert!(parser.parse_bool_value("invalid").is_err());
        assert!(parser.parse_bool_value("2").is_err());
        assert!(parser.parse_bool_value("").is_err());

        Ok(())
    }

    // Test quote-preserving string splitting
    #[test]
    fn test_split_preserving_quotes() -> Result<(), Box<dyn std::error::Error>> {
        let parser = FormfileParser::new();

        // Test basic splitting
        let result = parser.split_preserving_quotes("arg1 arg2 arg3")?;
        assert_eq!(result, vec!["arg1", "arg2", "arg3"]);

        // Test quoted strings
        let result = parser.split_preserving_quotes(r#"arg1 "arg 2" 'arg 3'"#)?;
        assert_eq!(result, vec!["arg1", "arg 2", "arg 3"]);

        // Test unclosed quotes
        assert!(parser.split_preserving_quotes(r#"arg1 "unclosed"#).is_err());

        Ok(())
    }

    #[test]
    fn test_password_hashing() -> Result<(), Box<dyn std::error::Error>> {
        let parser = FormfileParser::new();
        let password = "testpass123";
        let salt = "abcdefghijklmnop";
        let hashed_password = parser.hash_password(password, Some(salt))?;
        assert_eq!(hashed_password, "$6$rounds=4096$abcdefghijklmnop$6v80AV.KRtXR2FO0bBjg10CO0sbOQm3B/HKH4aivq1tDVMaKBZYQSkTjVywGMVmRMi8YhUl3Zqf/1J7Ib5yj7/");
        Ok(())
        
    }

    // Test username validation
    #[test]
    fn test_username_validation() -> Result<(), Box<dyn std::error::Error>> {
        let parser = FormfileParser::new();

        // Test valid usernames
        assert!(parser.validate_username("alice").is_ok());
        assert!(parser.validate_username("user123").is_ok());
        assert!(parser.validate_username("user-name").is_ok());
        assert!(parser.validate_username("_user").is_ok());

        // Test invalid usernames
        assert!(parser.validate_username("").is_err()); // Empty
        assert!(parser.validate_username("User").is_err()); // Uppercase
        assert!(parser.validate_username("123user").is_err()); // Starts with number
        assert!(parser.validate_username("user@name").is_err()); // Invalid character
        assert!(parser.validate_username(&"a".repeat(33)).is_err()); // Too long

        Ok(())
    }

    // Test complete Formfile parsing
    #[test]
    fn test_complete_formfile() -> Result<(), Box<dyn std::error::Error>> {
        let mut parser = FormfileParser::new();
        let content = r#"
        # System configuration
        VCPU 4
        MEMORY 2048
        DISK 20
        
        # User configuration
        USER username:webdev passwd:securepass123 sudo:yes groups:docker,developers
        
        # Environment variables
        ENV --scope=system PATH=/usr/local/bin
        ENV --scope=user:webdev DB_HOST=localhost
        
        # Build instructions
        COPY /src/app
        INSTALL nginx postgresql python3
        RUN pip install -r requirements.txt
        "#;

        let formfile = parser.parse(content)?;
        
        // Verify system configuration
        assert_eq!(formfile.system_config.len(), 3);
        
        // Verify user configuration
        assert_eq!(formfile.users.len(), 1);
        assert_eq!(formfile.users[0].username, "webdev");
        
        // Verify build instructions
        assert!(formfile.build_instructions.iter().any(|i| matches!(i, BuildInstruction::Copy(_))));
        assert!(formfile.build_instructions.iter().any(|i| matches!(i, BuildInstruction::Install(_))));
        assert!(formfile.build_instructions.iter().any(|i| matches!(i, BuildInstruction::Run(_))));

        Ok(())
    }

    #[test]
    fn test_entrypoint_empty_cases() {
        let mut parser = FormfileParser::new();
        
        // Test completely empty ENTRYPOINT
        let result = parser.parse_entrypoint("");
        println!("{result:?}");
        assert!(result.is_ok());
        assert!(parser.instructions.is_empty());
        
        // Test whitespace-only ENTRYPOINT
        let mut parser = FormfileParser::new();
        let result = parser.parse_entrypoint("   ");
        println!("{result:?}");
        assert!(result.is_ok());
        assert!(parser.instructions.is_empty());
        
        // Test empty JSON array
        let mut parser = FormfileParser::new();
        let result = parser.parse_entrypoint("[]");
        println!("{result:?}");
        assert!(result.is_ok());
        assert!(parser.instructions.is_empty());
        
        // Test JSON array with whitespace
        let mut parser = FormfileParser::new();
        let result = parser.parse_entrypoint("[    ]");
        println!("{result:?}");
        assert!(result.is_ok());
        assert!(parser.instructions.is_empty());
    }

    #[test]
    fn test_entrypoint_json_array_format() {
        let mut parser = FormfileParser::new();
        
        // Test valid JSON array format
        let result = parser.parse_entrypoint(r#"["npm", "/app/server.js"]"#);
        assert!(result.is_ok());
        
        if let Some(BuildInstruction::Entrypoint(entrypoint)) = parser.instructions.last() {
            assert_eq!(entrypoint.command, "npm");
            assert_eq!(entrypoint.args, vec!["/app/server.js"]);
        } else {
            panic!("Expected Entrypoint instruction");
        }

        // Test with multiple arguments
        let mut parser = FormfileParser::new();
        let result = parser.parse_entrypoint(r#"["/usr/bin/python3", "-m", "flask", "run"]"#);
        assert!(result.is_ok());
        
        if let Some(BuildInstruction::Entrypoint(entrypoint)) = parser.instructions.last() {
            assert_eq!(entrypoint.command, "/usr/bin/python3");
            assert_eq!(entrypoint.args, vec!["-m", "flask", "run"]);
        }
    }

    #[test]
    fn test_entrypoint_shell_format() {
        let mut parser = FormfileParser::new();
        
        // Test basic shell format
        let result = parser.parse_entrypoint("npm /app/server.js");
        assert!(result.is_ok());
        
        if let Some(BuildInstruction::Entrypoint(entrypoint)) = parser.instructions.last() {
            assert_eq!(entrypoint.command, "npm");
            assert_eq!(entrypoint.args, vec!["/app/server.js"]);
        }

        // Test with quoted arguments
        let mut parser = FormfileParser::new();
        let result = parser.parse_entrypoint(r#"python3 -m "flask run""#);
        assert!(result.is_ok());
        
        if let Some(BuildInstruction::Entrypoint(entrypoint)) = parser.instructions.last() {
            assert_eq!(entrypoint.command, "python3");
            assert_eq!(entrypoint.args, vec!["-m", "flask", "run"]);
        }
    }

    #[test]
    fn test_entrypoint_malformed_json() {
        let mut parser = FormfileParser::new();
        
        // Test malformed JSON that should error
        let result = parser.parse_entrypoint(r#"["unclosed"#);
        println!("Result: {result:?}");
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("missing closing bracket"),"Unexpected error message: {}", error);
        
        // Test invalid JSON content
        let result = parser.parse_entrypoint(r#"[not, valid, json]"#);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Invalid JSON in ENTRYPOINT array"), "Unexpected error message: {}", error);
    }
}
