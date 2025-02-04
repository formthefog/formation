use formnet_server::{db::CrdtMap, DatabaseCidr};
use shared::AddCidrOpts;

pub async fn add_cidr(
    opts: AddCidrOpts
) -> Result<(), Box<dyn std::error::Error>> {
    let cidrs = DatabaseCidr::<String, CrdtMap>::list().await?;
    if let Some(cidr_request) = shared::prompts::add_cidr(&cidrs, &opts)? {
        let cidr = DatabaseCidr::<String, CrdtMap>::create(cidr_request).await?;
        print!(
            r#"
            CIDR \"{cidr_name}\" added.

            Right now, peers within {cidr_name} can only see peers in the same CIDR, and in
            the special \"innernet-server\" CIDR that includes the innernet server peer.

            You'll need to add more associations for peers in diffent CIDRs to communicate.
            "#,
            cidr_name = cidr.name
        );
    } else {
        log::info!("exited without creating CIDR.");
    }

    Ok(())
}
