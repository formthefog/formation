pub fn generate_default_runcmds() -> Vec<String> {
    let mut cmds = vec![];
    cmds.push("sudo netplan apply".to_string());
    cmds.push("sudo systemctl daemon-reload".to_string());
    cmds.push("sudo systemctl enable formnet-install".to_string());
    cmds.push("sudo systemctl enable formnet-up".to_string());
    cmds.push("sudo systemctl start formnet-install".to_string());
    cmds.push("sudo systemctl start formnet-up".to_string());
   // cmds.push("sudo formnet install --default-name -d /etc/formnet/invite.toml".to_string());
   // cmds.push("sudo formnet up -d --interval 60".to_string());
    cmds
}
