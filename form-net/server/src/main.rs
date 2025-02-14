use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use shared::{
    AddCidrOpts, AddPeerOpts, DeleteCidrOpts, EnableDisablePeerOpts, NetworkOpts, RenameCidrOpts,
    RenamePeerOpts,
};
use std::{env, path::PathBuf};

use formnet_server::{
    db::{CrdtMap, Sqlite}, initialize::{self, InitializeOpts}, FormnetNode, ServerConfig
};
use shared::Interface;

#[derive(Clone, Debug, ValueEnum)]
pub enum Datastore {
    Sql,
    Crdt
}


#[derive(Debug, Parser)]
#[command(name = "innernet-server", author, version, about)]
struct Opts {
    #[clap(subcommand)]
    command: Command,

    #[clap(short, long, default_value = "/etc/innernet-server")]
    config_dir: PathBuf,

    #[clap(short, long, default_value = "/var/lib/innernet-server")]
    data_dir: PathBuf,

    #[clap(flatten)]
    network: NetworkOpts,

    #[clap(value_enum, short, long, default_value_t = Datastore::Crdt)]
    datastore: Datastore
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a new network.
    #[clap(alias = "init")]
    New {
        #[clap(flatten)]
        opts: InitializeOpts,
    },

    /// Permanently uninstall a created network, rendering it unusable. Use with care.
    Uninstall {
        interface: Interface,

        /// Bypass confirmation
        #[clap(long)]
        yes: bool,
    },

    /// Serve the coordinating server for an existing network.
    Serve {
        interface: Interface,

        #[clap(flatten)]
        network: NetworkOpts,
    },

    /// Add a peer to an existing network.
    AddPeer {
        interface: Interface,

        #[clap(flatten)]
        args: AddPeerOpts,
    },

    /// Disable an enabled peer
    DisablePeer {
        interface: Interface,

        #[clap(flatten)]
        args: EnableDisablePeerOpts,
    },

    /// Enable a disabled peer
    EnablePeer {
        interface: Interface,

        #[clap(flatten)]
        args: EnableDisablePeerOpts,
    },

    /// Rename an existing peer.
    RenamePeer {
        interface: Interface,

        #[clap(flatten)]
        args: RenamePeerOpts,
    },

    /// Add a new CIDR to an existing network.
    AddCidr {
        interface: Interface,

        #[clap(flatten)]
        args: AddCidrOpts,
    },

    /// Rename an existing CIDR.
    RenameCidr {
        interface: Interface,

        #[clap(flatten)]
        args: RenameCidrOpts,
    },

    /// Delete a CIDR.
    DeleteCidr {
        interface: Interface,

        #[clap(flatten)]
        args: DeleteCidrOpts,
    },

    /// Generate shell completion scripts
    Completions {
        #[clap(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::var_os("RUST_LOG").is_none() {
        // Set some default log settings.
        env::set_var("RUST_LOG", "warn,warp=info,wg_manage_server=info");
    }

    pretty_env_logger::init();
    let opts = Opts::parse();

    if unsafe { libc::getuid() } != 0 && !matches!(opts.command, Command::Completions { .. }) {
        return Err("innernet-server must run as root.".into());
    }

    let conf = ServerConfig::new(opts.config_dir, opts.data_dir);

    match opts.command {
        Command::New { opts } => {
            if let Err(e) = initialize::init_wizard(&conf, opts) {
                eprintln!("{}: {}.", "creation failed".red(), e);
                std::process::exit(1);
            }
        },
        Command::Uninstall { interface, yes } => {
            match opts.datastore {
                Datastore::Crdt => <CrdtMap as FormnetNode>::uninstall(&CrdtMap, &interface, &conf, opts.network, yes).await?,
                Datastore::Sql => <Sqlite as FormnetNode>::uninstall(&Sqlite, &interface, &conf, opts.network, yes).await?
            }
        }
        Command::Serve {
            interface,
            network: routing,
        } => {
            match opts.datastore {
                Datastore::Crdt => <CrdtMap as FormnetNode>::serve(*interface, &conf, routing).await?,
                Datastore::Sql => <Sqlite as FormnetNode>::serve(*interface, &conf, routing).await?
            }
        }
        Command::AddPeer { interface, args } => {
            match opts.datastore {
                Datastore::Crdt => <CrdtMap as FormnetNode>::add_peer(&CrdtMap, &interface, &conf, args, opts.network).await?,
                Datastore::Sql => <Sqlite as FormnetNode>::add_peer(&Sqlite, &interface, &conf, args, opts.network).await?
            }
        },
        Command::RenamePeer { interface, args } => {
            match opts.datastore {
                Datastore::Crdt => <CrdtMap as FormnetNode>::rename_peer(&CrdtMap, &interface, &conf, args).await?,
                Datastore::Sql => <Sqlite as FormnetNode>::rename_peer(&Sqlite, &interface, &conf, args).await?
            }
        },
        Command::DisablePeer { interface, args } => {
            match opts.datastore {
                Datastore::Crdt => <CrdtMap as FormnetNode>::enable_or_disable_peer(&CrdtMap, &interface, &conf, false, opts.network, args).await?,
                Datastore::Sql => <Sqlite as FormnetNode>::enable_or_disable_peer(&Sqlite, &interface, &conf, false, opts.network, args).await?
            }
        },
        Command::EnablePeer { interface, args } => {
            match opts.datastore {
                Datastore::Crdt => <CrdtMap as FormnetNode>::enable_or_disable_peer(&CrdtMap, &interface, &conf, true, opts.network, args).await?,
                Datastore::Sql => <Sqlite as FormnetNode>::enable_or_disable_peer(&Sqlite, &interface, &conf, true, opts.network, args).await?
            }
        },
        Command::AddCidr { interface, args } => {
            match opts.datastore {
                Datastore::Crdt => <CrdtMap as FormnetNode>::add_cidr(&CrdtMap, &interface, &conf, args).await?,
                Datastore::Sql => <Sqlite as FormnetNode>::add_cidr(&Sqlite, &interface, &conf, args).await?,
            }
        },
        Command::RenameCidr { interface, args } => {
            match opts.datastore {
                Datastore::Crdt => <CrdtMap as FormnetNode>::rename_cidr(&CrdtMap, &interface, &conf, args).await?,
                Datastore::Sql => <Sqlite as FormnetNode>::rename_cidr(&Sqlite, &interface, &conf, args).await?,
            }
        },
        Command::DeleteCidr { interface, args } => {
            match opts.datastore {
                Datastore::Crdt => <CrdtMap as FormnetNode>::delete_cidr(&CrdtMap, &interface, &conf, args).await?,
                Datastore::Sql => <Sqlite as FormnetNode>::delete_cidr(&Sqlite, &interface, &conf, args).await?,
            }
        }
        Command::Completions { shell } => {
            use clap::CommandFactory;
            let mut app = Opts::command();
            let app_name = app.get_name().to_string();
            clap_complete::generate(shell, &mut app, app_name, &mut std::io::stdout());
            std::process::exit(0);
        },
    }

    Ok(())
}
