use clap::Args;
use colored::Colorize;
use form_types::state::{Response as StateResponse, Success};
use form_state::instances::Instance;
use reqwest::Client;

/// Acquires the status of a build and it's instances.
#[derive(Debug, Clone, Args)]
pub struct StatusCommand {
    /// This is the build ID that you received as part of the response
    /// from the `form pack build` command.
    /// If you lost it you can call `form pack get-build-id` from the 
    /// build context directory (the same directory as your Formfile)
    /// and `form` will derive it from your formfile and your
    /// signing key.
    #[clap(long="build-id", short='i')]
    build_id: String
}

impl StatusCommand {
    pub async fn handle_status(&self, provider: String, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let status = Client::new()
            .post(&format!("http://{provider}:{port}/instance/{}/get_by_build_id", self.build_id))
            .send().await?
            .json::<StateResponse<Instance>>()
            .await?;

        print_pack_status(status, self.build_id.clone());

        Ok(())
    }
}

pub fn print_pack_status(status: StateResponse<Instance>, build_id: String) {
    match status {
        StateResponse::Success(
            Success::List(instances)
        ) => {
            let n = instances.len();
            let info = instances.iter().map(|inst| {
                vec![inst.node_id.clone(),
                inst.instance_id.clone(),
                inst.status.to_string().clone()]
            }).collect::<Vec<Vec<String>>>();

            let display = info.iter().map(|inner| {
                format!(r#"
                -----------------------------------------------------------
                {}: {}
                {}: {}
                {}: {}

                "#,
                "Instance ID:".bold().bright_cyan(), 
                inner[0].bright_yellow(), 
                "Hosted On:".bold().bright_cyan(),
                inner[1].bright_yellow(), 
                "Status:".bold().bright_cyan(), 
                inner[2].bright_yellow())
            }).collect::<Vec<String>>();

            println!(
r#"
We were able to acquire the status of your build with {} {}.

{} has {} instances, below is the statuses of each:

{}

You no longer have to {} to {} your build.

We have recently enhanced our Virtual Machine Manager Protocol to {} 
builds to {} if they haven't already.

If at least {} of your {} is in the {} phase, you can run:

```
{} {} {}
```

and continue to poll for it's {} using this same command.

When the status is "{}" you can run 

```
{} {} {}
```

to get the {} for your {}. Once you have your {} {} you can ssh into it ({}) with:

```
{}
```
{}
"#,
"Build ID:".bold().bright_cyan(),
build_id.yellow(),
build_id.yellow(),
n.to_string().bold().bright_blue(),
display.join("\n"),
"wait".bright_yellow(),
"ship".bright_green(),
"watch for".bright_yellow(),
"finish".bright_blue(),
"1".bright_green(),
"instances".bright_yellow(),
"building".bright_blue(),
"form".bright_blue(),
"[OPTIONS]".yellow(),
"pack ship".bright_blue(),
"status".bright_cyan(),
"Started".bright_green(),
"form".bright_blue(),
"[OPTIONS]".yellow(),
"manage get-ip".bright_blue(),
"formnet IP addresses".bright_magenta(),
"instances".bright_yellow(),
"instance".bright_yellow(),
"formnet IP address".bright_magenta(),
"assuming you have joined `formnet`".bold().bright_red(),
"ssh <username>@<formnet-ip>".bright_blue(),
"assuming you included your ssh keys in your Formfile".bold().bright_red(),
);
        }
        StateResponse::Failure { reason } => {
            println!(
r#"Unfortunately we were unable to acquire the status of your instance 
with {} {}"
"#,
"Build ID:".bold().bright_cyan(),
build_id.bright_yellow(),
);
            if reason.is_some() {
                println!(
r#"

{}: {}
"#,
"Reason".bold().bright_cyan(),
reason.unwrap().bright_yellow()
);
            } else {
                println!(
r#"
Unfortunately, no reasson for the failure was returned.
"#,
);
            }
            println!(
r#"
We understand this can be frustrating. We want to help.

Please consider doing one of the following: 

    1. Join our discord at {} and go to the {} channel and paste this response
    2. Submitting an {} on our project github at {} 
    3. Sending us a direct message on X at {}

Someone from our core team will gladly help you out.
"#,
"discord.gg/formation".underline().blue(),
"#chewing-glass".blue(),
"issue".bright_yellow(),
"http://github.com/formthefog/formation.git".blue(),
"@formthefog".blue(),
);
        }
        _ => {
            println!(r#"
Something went {} wrong. The response received was {:?} which is an invalid response 
to the `{}` command.

Please consider doing one of the following: 

    1. Join our discord at {} and go to the {} channel and paste this response
    2. Submitting an {} on our project github at {} 
    3. Sending us a direct message on X at {}

Someone from our core team will gladly help you out.
"#,
"terribly".bright_red().on_blue(),
status,
"form pack [OPTIONS] build".bright_yellow(),
"discord.gg/formation".blue(),
"chewing-glass".blue(),
"issue".bright_yellow(),
"http://github.com/formthefog/formation.git".blue(),
"@formthefog".blue(),
);
        }
    }
}
