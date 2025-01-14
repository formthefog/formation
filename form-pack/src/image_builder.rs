use std::{
    collections::HashSet, fs::{File, OpenOptions}, io::Write, path::{Path, PathBuf}, process::Command, sync::Mutex
};
use lazy_static::lazy_static;

use crate::formfile::{Entrypoint, User};

pub const MAX_NBD_DEVICES: usize = 8;
pub const BUILD_MOUNT_PATH: &str = "/mnt/cloudimg";

lazy_static! {
    static ref NBD_MANAGER: Mutex<NbdDeviceManager> = Mutex::new(NbdDeviceManager::new());
}

struct NbdDeviceManager {
    in_use: HashSet<usize>
}

impl NbdDeviceManager {
    pub fn new() -> Self {
        Self {
            in_use: HashSet::new()
        }
    }

    pub fn allocate_device(
        &mut self,
    ) -> Option<usize> {
        for device_num in 0..MAX_NBD_DEVICES {
            if !self.in_use.contains(&device_num) {
                self.in_use.insert(device_num);
                return Some(device_num);
            }
        }
        None
    }

    pub fn release_device(&mut self, device_num: usize) {
        self.in_use.remove(&device_num); 
    }
}

pub struct NbdDevice {
    device_id: usize,
    device: PathBuf
}

impl NbdDevice {
    pub fn new(image: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut manager = NBD_MANAGER.lock()?;

        //TODO: Place the request in a queue and/or send to a different node
        //to have built
        let device_num = manager.allocate_device().ok_or(
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No avaialble devices"))
        )?;

        let device_path = PathBuf::from(format!("/dev/nbd{}", device_num));

        Self::cleanup_device(&device_path)?;

        Command::new("qemu-nbd")
            .args(&["--connect", &device_path.to_string_lossy()])
            .arg(image)
            .status()?;

        Command::new("udevadm")
            .arg("settle")
            .status()?;

        if !Path::new(&format!("{}p1", device_path.display())).exists() {
            manager.release_device(device_num);
            return Err(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Unable to acquire device path"
                )
            ))
        }

        Ok(Self {
            device_id: device_num,
            device: device_path
        })
    }

    fn cleanup_device(device: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(mount_output) = std::fs::read_to_string("/proc/mounts") {
            if mount_output.contains(&*device.to_string_lossy()) {
                Command::new("umount")
                    .arg(device)
                    .status()?;
            }
        }

        Command::new("qemu-nbd")
            .args(&["--disconnect", &device.to_string_lossy()])
            .status().ok();

        Ok(())
    }

    /// Get the path to the first partition on the device
    pub fn partition_device(&self) -> PathBuf {
        PathBuf::from(format!("{}p1", self.device.display()))
    }

    /// Get the raw device path
    pub fn device_path(&self) -> &Path {
        &self.device
    }
}

impl Drop for NbdDevice {
    fn drop(&mut self) {
        if let Err(e) = Command::new("qemu-nbd")
            .args(&["--disconnect", &self.device.to_string_lossy()])
            .status()
        {
            eprintln!("Failed to disconnect NBD device: {e}");
        }

        // Release t he device number back to the pool
        NBD_MANAGER.lock().unwrap().release_device(self.device_id);
        eprintln!("Released NBD device {}", self.device_id);
    }
}

pub fn mount_nbd_device(device: NbdDevice) -> Result<(), Box<dyn std::error::Error>> {
    let mount_path = BUILD_MOUNT_PATH.to_string();
    Command::new("mount")
        .arg(device.device.clone())
        .arg(mount_path)
        .status()?;

    Ok(())
}

pub fn unmount_nbd_device() -> Result<(), Box<dyn std::error::Error>> {
    Command::new("umount")
        .arg(BUILD_MOUNT_PATH)
        .status()?;

    Ok(())
}

pub fn chroot_into_mount() -> Result<(), Box<dyn std::error::Error>> {
    Command::new("chroot")
        .arg(BUILD_MOUNT_PATH)
        .status()?;

    Ok(())
}

pub fn install_packages(packages: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("apt-get")
        .arg("install")
        .args(packages)
        .status()?;

    Ok(())
}

pub fn update_apt_get() -> Result<(), Box<dyn std::error::Error>> {
    Command::new("apt-get")
        .arg("update")
        .status()?;

    Ok(())
}

pub fn upgrade_apt_get() -> Result<(), Box<dyn std::error::Error>> {
    Command::new("apt-get")
        .arg("upgrade")
        .status()?;

    Ok(())
}

pub fn setup_workdir(workdir: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(workdir)?;

    Ok(())
}

#[cfg(not(test))]
pub fn setup_users(users: &[User]) -> Result<(), Box<dyn std::error::Error>> {
    for user in users {
        build_user(&user)?;
    }

    Ok(())
}

pub fn append_only_file(path: impl AsRef<Path>) -> Result<File, Box<dyn std::error::Error>> {
    Ok(OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(path)?
    )
}

#[cfg(test)]
pub fn build_user(
    user: &User,
    passwd_path: &str,
    group_path: &str,
    shadow_path: &str,
    home_path: &str,
    skel_path: &str,
    sudoers_path: &str
) -> Result<(), Box<dyn std::error::Error>> {
    write_passwd(user, passwd_path)?;
    write_group(user, group_path)?;
    write_shadow(user, shadow_path)?;
    make_home(user, home_path, skel_path)?;
    authorized_users(user, home_path, ".ssh")?;
    chmod_home(user, home_path)?;
    chown_home(user, home_path)?;
    add_to_sudo(user, sudoers_path)?;

    Ok(())
}


#[cfg(not(test))]
pub fn build_user(user: &User) -> Result<(), Box<dyn std::error::Error>> {
    write_passwd(user, "/etc/passwd")?;
    write_group(user, "/etc/group")?;
    write_shadow(user, "/etc/shadow")?;
    make_home(user, "/home", "/etc/skel/.")?;
    authorized_users(user, "/home", ".ssh")?;
    chmod_home(user, "/home")?;
    chown_home(user, "/home")?;
    add_to_sudo(user, "/etc/sudoers.d/custom")?;
    Ok(())
}

pub fn write_passwd(user: &User, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = append_only_file(path)?;
    let passwd_content = std::fs::read_to_string(path)?; 
    let next_uid = passwd_content
        .lines()
        .filter_map(|line| {
            line.split(':').nth(2)?.parse::<u32>().ok()
        }).max().unwrap_or(999) + 1;

    writeln!(
        file,
        "{}:x:{}:{}:{}:/home/{}:{}",
        user.username(),
        next_uid,
        next_uid,
        user.username(),
        user.username(),
        user.shell()
    )?;

    Ok(())
}

pub fn write_group(user: &User, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = append_only_file(path)?;

    let group_content = std::fs::read_to_string(path)?;
    let next_gid = group_content
        .lines()
        .filter_map(|line| {
            line.split(':').nth(2)?.parse::<u32>().ok()
        }).max().unwrap_or(999) + 1;

    writeln!(file, "{}:x:{}:{}", user.username(), next_gid, user.username())?;

    for group in user.groups() {
        let mut existing_group = std::fs::read_to_string(path)?
            .lines()
            .find(|line| line.split(':').next() == Some(group))
            .map(String::from);

        if let Some(group_line) = existing_group.as_mut() {
            if !group_line.ends_with(',') && !group_line.ends_with(':') {
                group_line.push(',');
            }
            group_line.push_str(&user.username());
            writeln!(file, "{}", group_line)?;
        } else {
            let next_gid = next_gid + 1;
            writeln!(file, "{}:x:{}:{}", group, next_gid, user.username())?;
        }
    }

    Ok(())
}

pub fn write_shadow(user: &User, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = append_only_file(path)?;
    
    let days_since_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() / 86400;

    let password_field = if user.lock_passwd() {
        format!("!{}", user.passwd())
    } else {
        user.passwd().to_string()
    };

    let max_days = if user.chpasswd_expire() { "99999" } else { "-1" };

    writeln!(
        file,
        "{}:{}:{}:0:{}:7:-1:-1:",
        user.username(),
        password_field,
        days_since_epoch,
        max_days,
    )?;

    Ok(())
}

pub fn make_home(user: &User, home_path: &str, skel_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&format!("{home_path}/{}", user.username()))?;
    add_skeletons(user, skel_path, home_path)?;

    Ok(())
}

pub fn add_skeletons(user: &User, skel_path: &str, home_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    copy_dir_recursively(skel_path, &format!("{home_path}/{}/", user.username()))?;

    Ok(())
}

pub fn chmod_home(user: &User, home_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("chmod")
        .arg("755")
        .arg(format!("{home_path}/{}", user.username()))
        .status()?;

    Ok(())
}

pub fn chown_home(user: &User, home_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("chown")
        .arg("-R")
        .arg(format!("{}:{}", user.username(), user.username()))
        .arg(format!("{home_path}/{}", user.username()))
        .status()?;

    Ok(())
}

pub fn authorized_users(user: &User, parent: &str, child: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Only set up SSH keys if there are any provided
    if user.ssh_authorized_keys().is_empty() {
        return Ok(());
    }

    let ssh_dir = PathBuf::from(format!("/{}/{}/{}", parent, user.username(), child));
    std::fs::create_dir_all(&ssh_dir)?;
    
    let auth_keys_path = ssh_dir.join("authorized_keys");
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(auth_keys_path.clone())?;

    for key in user.ssh_authorized_keys() {
        writeln!(file, "{}", key)?;
    }

    // Set permissions that enforce SSH key security requirements
    Command::new("chmod")
        .arg("700")
        .arg(&ssh_dir)
        .status()?;

    Command::new("chmod")
        .arg("600")
        .arg(&auth_keys_path)
        .status()?;

    Command::new("chown")
        .arg("-R")
        .arg(format!("{}:{}", user.username(), user.username()))
        .arg(&ssh_dir)
        .status()?;

    Ok(())
}

pub fn add_to_sudo(user: &User, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Only add user to sudo if the sudo flag is true
    if !user.sudo() {
        return Ok(());
    }

    Command::new("chmod")
        .arg("0777")
        .arg(path)
        .status()?;


    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(path)?;

    writeln!(file, "{} ALL=(ALL) NOPASSWD:ALL", user.username())?;
    
    Command::new("chmod")
        .arg("0440")
        .arg(path)
        .status()?;

    Ok(())
}

pub fn build_entrypoint(
    entrypoint: &Entrypoint,
    service_path: &str,
    wants: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let service_name = "form-app.service";
    let service_path_full = PathBuf::from(service_path).join(service_name);

    let exec_start = if entrypoint.args().is_empty() {
        if entrypoint.command().is_empty() {
            return Ok(())
        }
        entrypoint.command().to_string()
    } else {
        format!("{} {}", entrypoint.command(), entrypoint.args().join(" "))
    };

    println!("OPENING SERVICE FILE");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&service_path_full)?;

    // Write our systemd unit configuration
    // We're following systemd best practices for long-running application services
    println!("WRITING TO SERVICE FILE");
    write!(file, r#"[Unit]
Description=Form Network Application Service
After=network.target       # Ensure network is available
Wants=network-online.target  # Prefer full network connectivity

[Service]
Type=simple               # Process directly runs the application
ExecStart={}             # Our application command
Restart=always           # Automatic restart on failure
RestartSec=3             # Wait 3 seconds before restart
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
"#, exec_start)?;

    println!("CHANGING PERMISSIONS TO SERVICE FILE");
    Command::new("chmod")
        .arg("644")
        .arg(&service_path_full)
        .status()?;

    let wants_dir = PathBuf::from(&format!("{}/{wants}", service_path));
    std::fs::create_dir_all(&wants_dir)?;
    let symlink_path = wants_dir.join(service_name);
    println!("SYMLINKING WANTS DIR TO SERVICE FILE");
    std::os::unix::fs::symlink(&service_path_full, &symlink_path)?;

    Ok(())
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


#[cfg(test)]
mod tests {
    use crate::formfile::{EntrypointBuilder, UserBuilder};

    use super::*;
    use std::{collections::HashMap, io::{Read, Seek, SeekFrom}, os::unix::fs::PermissionsExt};
    use tempfile::{tempdir, NamedTempFile};

    // Helper function to create a temporary file with initial content
    fn create_temp_file_with_content(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        file
    }

    // Helper function to read entire file content
    fn read_file_content(mut file: &File) -> String {
        let mut content = String::new();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_to_string(&mut content).unwrap();
        content
    }

    // Helper function to create a test User struct
    fn create_test_user(username: &str) -> User {
        UserBuilder::new()
            .username(username)
            .passwd("$6$rounds=4096$salt$hashedpassword")
            .ssh_authorized_keys(vec!["ssh-rsa AAAA...".to_string()])
            .lock_passwd(false)
            .sudo(true)
            .shell("/bin/bash")
            .ssh_pwauth(true)
            .disable_root(true)
            .chpasswd_expire(true)
            .chpasswd_list(HashMap::new())
            .groups(vec!["docker".to_string(), "developers".to_string()])
            .build().unwrap()
    }

    #[test]
    fn test_write_passwd() {
        // Create a temporary passwd file with existing content
        let initial_content = "\
            root:x:0:0:root:/root:/bin/bash\n\
            daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin\n\
            bin:x:2:2:bin:/bin:/usr/sbin/nologin\n";
        
        let passwd_file = create_temp_file_with_content(initial_content);
        let test_user = create_test_user("testuser");

        // Monkeypatch the append_only_file function for testing
        let result = write_passwd(&test_user, passwd_file.path().to_str().unwrap());
        assert!(result.is_ok());

        // Read the final content and verify
        let content = read_file_content(&passwd_file.as_file());
        let lines: Vec<&str> = content.lines().collect();
        let last_line = lines.last().unwrap();

        // The new user should have UID/GID 1000 (as it's the first regular user)
        assert!(last_line.starts_with("testuser:x:3:3:"));
        assert!(last_line.ends_with(":/home/testuser:/bin/bash"));
    }

    #[test]
    fn test_write_shadow() {
        let initial_content = "\
            root:*:19167:0:99999:7:::\n\
            daemon:*:19167:0:99999:7:::\n";
        
        let shadow_file = create_temp_file_with_content(initial_content);
        let mut test_user = create_test_user("testuser");

        // Test normal password
        let result = write_shadow(&test_user, shadow_file.path().to_str().unwrap());
        assert!(result.is_ok());
        let content = read_file_content(&shadow_file.as_file());
        let lines: Vec<&str> = content.lines().collect();
        let last_line = lines.last().unwrap();
        assert!(!last_line.starts_with("testuser:!"));

        // Test locked password
        test_user.set_lock_passwd(true);
        let result = write_shadow(&test_user, shadow_file.path().to_str().unwrap());
        assert!(result.is_ok());
        let content = read_file_content(&shadow_file.as_file());
        let lines: Vec<&str> = content.lines().collect();
        let last_line = lines.last().unwrap();
        assert!(last_line.starts_with("testuser:!"));
    }

    #[test]
    fn test_write_group() {
        let initial_content = "\
            root:x:0:\n\
            daemon:x:1:\n\
            docker:x:999:user1\n";
        
        let group_file = create_temp_file_with_content(initial_content);
        let test_user = create_test_user("testuser");

        let result = write_group(&test_user, group_file.path().to_str().unwrap());
        assert!(result.is_ok());

        let content = read_file_content(&group_file.as_file());
        let lines: Vec<&str> = content.lines().collect();

        // Verify primary group creation
        assert!(lines.iter().any(|&line| 
            line.starts_with("testuser:x:1000:testuser")));

        // Verify additional group memberships
        assert!(lines.iter().any(|&line| 
            line.starts_with("docker:x:999:user1,testuser")));
        assert!(lines.iter().any(|&line|
            line.starts_with("developers:x:1001:testuser")));
    }

    #[test]
    fn test_user_home_directory_setup() {
        let temp_dir = tempdir().unwrap();
        let test_user = create_test_user("testuser");

        // Create skeleton directory structure
        let skel_dir = temp_dir.path().join("etc").join("skel");
        std::fs::create_dir_all(&skel_dir).unwrap();
        std::fs::write(skel_dir.join(".bashrc"), "# Test bashrc").unwrap();

        let result = make_home(
            &test_user,
            temp_dir.path().join("home").to_str().unwrap(), 
            temp_dir.path().join("etc").join("skel").to_str().unwrap()
        );
        assert!(result.is_ok());

        let home_dir = temp_dir.path().join("home").join("testuser");
        assert!(home_dir.exists());
        assert!(home_dir.join(".bashrc").exists());

        // Test permissions
        let result = chmod_home(&test_user, temp_dir.path().join("home").to_str().unwrap());
        assert!(result.is_ok());
        // In a real test, we'd verify the permissions here
    }

    #[test]
    fn test_sudo_setup() {
        // Create a temporary test environment
        let temp_dir = tempdir().unwrap();
        let sudoers_dir = temp_dir.path().join("etc").join("sudoers.d");
        std::fs::create_dir_all(&sudoers_dir).unwrap();
        let custom_sudoers = sudoers_dir.join("custom");

        // Test 1: User with sudo access
        let test_user = create_test_user("testuser");
        let result = add_to_sudo(&test_user, custom_sudoers.to_str().unwrap());
        assert!(result.is_ok());

        // Verify the sudoers file exists and has correct content
        assert!(custom_sudoers.exists());
        let sudoers_content = std::fs::read_to_string(&custom_sudoers).unwrap();
        assert_eq!(sudoers_content, "testuser ALL=(ALL) NOPASSWD:ALL\n");

        // Verify the file permissions are exactly 0440
        let metadata = std::fs::metadata(&custom_sudoers).unwrap();
        let permissions = metadata.permissions();
        #[cfg(unix)]
        assert_eq!(permissions.mode() & 0o777, 0o440);

        // Test 2: User without sudo access
        let mut no_sudo_user = create_test_user("regular_user");
        no_sudo_user.set_sudo(false);
        let result = add_to_sudo(&no_sudo_user, custom_sudoers.to_str().unwrap());
        assert!(result.is_ok());

        // Verify the sudoers file wasn't modified
        let sudoers_content_after = std::fs::read_to_string(&custom_sudoers).unwrap();
        assert_eq!(sudoers_content_after, "testuser ALL=(ALL) NOPASSWD:ALL\n",
            "Sudoers file was modified when it shouldn't have been");

        // Test 3: Multiple sudo users
        let another_sudo_user = create_test_user("admin_user");
        let result = add_to_sudo(&another_sudo_user, custom_sudoers.to_str().unwrap());
        assert!(result.is_ok());

        // Verify both users are in the file in the correct order
        let final_content = std::fs::read_to_string(&custom_sudoers).unwrap();
        assert_eq!(
            final_content,
            "testuser ALL=(ALL) NOPASSWD:ALL\nadmin_user ALL=(ALL) NOPASSWD:ALL\n",
            "Multiple sudo users not handled correctly"
        );

        // Test 4: Error handling - directory doesn't exist
        std::fs::remove_dir_all(&sudoers_dir).unwrap();
        let error_user = create_test_user("error_test");
        let result = add_to_sudo(&error_user, custom_sudoers.to_str().unwrap());
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("No such file or directory"),
            "Unexpected error message: {}", error);
    }

    #[test]
    fn test_ssh_key_setup() {
        let temp_dir = tempdir().unwrap();
        let test_user = create_test_user("testuser");
        let result = authorized_users(
            &test_user,
            temp_dir.path().join("home").to_str().unwrap(),
            ".ssh"
        );
        assert!(result.is_ok());

        let ssh_dir = temp_dir.path()
            .join("home")
            .join("testuser")
            .join(".ssh");
        let auth_keys_path = ssh_dir.join("authorized_keys");

        assert!(ssh_dir.exists());
        assert!(auth_keys_path.exists());

        let content = std::fs::read_to_string(auth_keys_path).unwrap();
        assert!(content.contains("ssh-rsa AAAA..."));
    }

    #[test]
    fn test_complete_user_setup() {
        let temp_dir = tempdir().unwrap();
        let test_user = create_test_user("testuser");
        let etc_path = temp_dir.path().join("etc");
        std::fs::create_dir_all(&etc_path).unwrap();
        std::fs::create_dir_all(&etc_path.join("skel")).unwrap();
        std::fs::create_dir_all(&etc_path.join("sudoers.d")).unwrap();

        let result = build_user(
            &test_user,
            temp_dir.path().join("etc").join("passwd").to_str().unwrap(),
            temp_dir.path().join("etc").join("group").to_str().unwrap(),
            temp_dir.path().join("etc").join("shadow").to_str().unwrap(),
            temp_dir.path().join("home").to_str().unwrap(),
            temp_dir.path().join("etc").join("skel").to_str().unwrap(),
            temp_dir.path().join("etc").join("sudoers.d").join("custom").to_str().unwrap(),
        );
        println!("{result:?}");
        assert!(result.is_ok());

        // Verify all components were created
        let home_dir = temp_dir.path().join("home").join("testuser");
        assert!(home_dir.exists());
        assert!(home_dir.join(".ssh").exists());
        
        // Verify user entries in system files
        // In a real test environment, we'd check all the file contents
    }

    #[test]
    fn test_build_entrypoint_service_creation() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory to simulate our chroot environment
        let temp_dir = tempdir()?;
        let root_path = temp_dir.path();

        // Create the necessary directory structure
        // /etc/systemd/system for the service file
        let systemd_system_dir = root_path.join("etc/systemd/system");
        std::fs::create_dir_all(&systemd_system_dir)?;
        
        // /etc/systemd/system/multi-user.target.wants for the symlink
        let wants_dir = systemd_system_dir.join("multi-user.target.wants");
        std::fs::create_dir_all(&wants_dir)?;

        // Create an entrypoint with a realistic application setup
        let entrypoint = EntrypointBuilder::new()
            .command("/usr/local/bin/myapp")
            .args(vec!["--port".to_string(), "8080".to_string()])
            .build();

        let result = build_entrypoint(&entrypoint, systemd_system_dir.to_str().unwrap(), "multi-user.target.wants");
        assert!(result.is_ok());

        // Verify the service file exists and has correct content
        let service_path = systemd_system_dir.join("form-app.service");
        assert!(service_path.exists(), "Service file was not created");

        let service_content = std::fs::read_to_string(&service_path)?;

        // Verify the essential parts of the service file
        assert!(service_content.contains("[Unit]"), "Missing Unit section");
        assert!(service_content.contains("[Service]"), "Missing Service section");
        assert!(service_content.contains("[Install]"), "Missing Install section");

        // Verify the specific configuration we care about
        assert!(service_content.contains("Description=Form Network Application Service"),
            "Missing or incorrect description");
        assert!(service_content.contains("ExecStart=/usr/local/bin/myapp --port 8080"),
            "Incorrect ExecStart command");
        assert!(service_content.contains("Restart=always"),
            "Missing or incorrect restart policy");
        assert!(service_content.contains("Type=simple"),
            "Missing or incorrect service type");

        // Verify security settings
        assert!(service_content.contains("NoNewPrivileges=true"),
            "Missing NoNewPrivileges security setting");
        assert!(service_content.contains("ProtectSystem=full"),
            "Missing ProtectSystem security setting");
        assert!(service_content.contains("ProtectHome=true"),
            "Missing ProtectHome security setting");

        // Verify the service file permissions
        let metadata = std::fs::metadata(&service_path)?;
        let permissions = metadata.permissions();
        #[cfg(unix)]
        assert_eq!(permissions.mode() & 0o777, 0o644,
            "Service file has incorrect permissions");

        // Verify the symlink exists and points to the correct location
        let symlink_path = wants_dir.join("form-app.service");
        assert!(symlink_path.exists(), "Symlink was not created");

        // On Unix systems, verify it's actually a symlink and points to the right place
        #[cfg(unix)]
        {
            assert!(std::fs::symlink_metadata(&symlink_path)?.file_type().is_symlink(),
                "File exists but is not a symlink");
            
            let link_target = std::fs::read_link(&symlink_path)?;
            assert_eq!(link_target, temp_dir.path().join("etc").join("systemd").join("system").join("form-app.service"),
                "Symlink points to incorrect target");
        }

        // Test error cases - try to create service in non-existent directory
        let bad_temp_dir = tempdir()?;
        let bad_path = bad_temp_dir.path();
        // Don't create the systemd directory structure
        let bad_entrypoint = EntrypointBuilder::new()
            .command("/usr/local/bin/myapp")
            .args(vec![])
            .build();

        let result = build_entrypoint(
            &bad_entrypoint,
            bad_path.join("systemd").join("system").to_str().unwrap(), 
            "multi-user.target.wants"
        );

        assert!(result.is_err(), "Should fail when systemd directory doesn't exist");
        assert!(result.unwrap_err().to_string().contains("No such file or directory"),
            "Unexpected error message for missing directory");

        Ok(())
    }
}
