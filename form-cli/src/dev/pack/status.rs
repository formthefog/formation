use clap::Args;
use colored::Colorize;
use form_pack::manager::{PackBuildStatus, PackResponse};
use reqwest::Client;

#[derive(Debug, Clone, Args)]
pub struct StatusCommand {
    #[clap(long="build-id", short='i')]
    build_id: String
}

impl StatusCommand {
    pub async fn handle_status(&self, provider: String, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let status = Client::new()
            .post(&format!("http://{provider}:{port}/{}/get_status", self.build_id))
            .send().await?
            .json::<PackResponse>()
            .await?;

        print_pack_status(status, self.build_id.clone());

        Ok(())
    }
}

pub fn print_pack_status(status: PackResponse, build_id: String) {
    match status {
        PackResponse::Status(
            PackBuildStatus::Started(_id)
        ) => {
            println!(r#"
Your build has {} but has not yet {}.

Please be patient, as builds can take as long as {}, or possibly even
longer depending on the size and number build artifacts and/or number of arguments in your
{} or options selected on the GUI. This is particularly true if you included a significant number 
of system dependencies or application level dependencies to be installed, or included large 
system dependencies to be installed.

Check back in a couple more minutes to see an update of the status.

If your status is still `{}` after {} please consider taking one of the following
actions:

    1. Join our discord at {} and go to the {} channel and paste this response
    2. Submitting an {} on our project github at {} 
    3. Sending us a direct message on X at {}

One of our core contributors will be glad to help you out.
"#,
"Started".blue(),
"Completed".yellow(),
"5 minutes".bright_purple(),
"Formfile".blue(),
"Started".yellow(),
"10 minutes".bright_red(),
"discord.gg/formation".blue(),
"chewing-glass".bright_yellow(),
"Issue".yellow(),
"http://github.com/formthefog/formation.git".blue(),
"@formthefog".blue(),
);
        }
        PackResponse::Status(
            PackBuildStatus::Failed { build_id, reason }
        ) => {
            println!(r#"
We regret to inform you that your build with build id {} has {}.

The reason for the falure was: {}

If the reason for the failure is bewildering or does not make sense to you, please
consider taking one of the following actions:

    1. Join our discord at {} and go to the {} channel and paste this response
    2. Submitting an {} on our project github at {} 
    3. Sending us a direct message on X at {}

One of our core contributors will be glad to help you out.
"#,
build_id.bright_blue(),
"Failed".bright_red(),
reason.italic().bright_magenta(),
"discord.gg/formation".blue(),
"chewing-glass".bright_yellow(),
"Issue".yellow(),
"http://github.com/formthefog/formation.git".blue(),
"@formthefog".blue(),
);
        }
        PackResponse::Status(
            PackBuildStatus::Completed(instance)
        ) => {
            println!(r#"
We are overjoyed to inform you that your build with build id {} has {}.

You can now `{}` your build by running:

```
{}
```

If you run into any issues during the `{}` phase of deployment, please consider
taking one of the following actions:

    1. Join our discord at {} and go to the {} channel and paste this response
    2. Submitting an {} on our project github at {} 
    3. Sending us a direct message on X at {}

One of our core contributors will be glad to help you out.
"#,
instance.build_id.bright_blue(),
"Completed".bright_red(),
"ship".blue(),
"form pack [OPTIONS] ship".bright_yellow(),
"ship".blue(),
"discord.gg/formation".blue(),
"chewing-glass".bright_yellow(),
"Issue".yellow(),
"http://github.com/formthefog/formation.git".blue(),
"@formthefog".blue(),
);
        }
        PackResponse::Failure => {
            println!(r#"
Something went wrong attempting to get the status for your build 
with build id: {}.

We are not exactly sure what happened, but the best steps to debug this
are as follows:

    1. Double check your `{}` to ensure they are still servicing the network
        1.a. If not, you may want to consider rebuilding your `{}` to select a new provider
        1.b. If so, consider that they may have reconfigured their node and are using a different port than the standard port
    2. Your provider may be faulty, you can submit a fault challenge. To do so see our docs at {} to learn more about how to do so.
    3. You may be using an outdated version of the form CLI. If so you may want to consider upgrading. Check for the latest relsease
    at our github at {}.
    4. Join our discord at {} and go to the {} channel and paste this response
    5. Submitting an {} on our project github at {} 
    6. Sending us a direct message on X at {}
"#,
build_id.blue(),
"provider".bright_yellow(),
"form kit".bright_magenta(),
"http://docs.formation.cloud/#fault-challenge".bright_blue(),
"http://github.com/formthefog/formation.git".blue(),
"discord.gg/formation".blue(),
"chewing-glass".bright_yellow(),
"Issue".yellow(),
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
