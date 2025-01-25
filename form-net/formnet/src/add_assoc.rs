use formnet_server::{db::{CrdtMap, DatabaseAssociation}, DatabaseCidr};
use shared::{AssociationContents, Cidr};

pub async fn add_association(
    cidr_1: String,
    cidr_2: String
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Fetching CIDRs");
    let cidrs: Vec<Cidr<String>> = DatabaseCidr::<String, CrdtMap>::list().await?;

    cidrs
        .iter()
        .find(|c| c.name == cidr_1)
        .ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("can't find cidr '{}'", cidr_1))))?;
    cidrs
        .iter()
        .find(|c| c.name == cidr_2)
        .ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("can't find cidr '{}'", cidr_2))))?;

    let contents = AssociationContents {
        cidr_id_1: cidr_1,
        cidr_id_2: cidr_2
    };
    DatabaseAssociation::<CrdtMap, String, String>::create(contents).await?;

    Ok(())
}

