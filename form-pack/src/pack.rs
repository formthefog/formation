use std::{path::{Path, PathBuf}, fs, io};
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder;
use serde::{Serialize, Deserialize};
use crate::{formfile::Formfile, image_builder::copy_dir_recursively};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormPack {
    pub formfile: Formfile,
    pub artifacts: Vec<u8>,
}

pub struct Pack {
    context_dir: PathBuf,
    artifact_dir: PathBuf,
}

impl Pack {
    pub fn new(context_dir: impl AsRef<Path>) -> Result<Self, io::Error> {
        let context = context_dir.as_ref().to_path_buf();

        let artifact_dir = tempfile::tempdir()?.into_path();

        Ok(Self {
            context_dir: context,
            artifact_dir,
        })
    }

    pub fn prepare_artifacts(
        &self,
        copy_instructions: &[(PathBuf, PathBuf)],
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        if copy_instructions.is_empty() {
            println!("No COPY instructions provided in Formfile...");
            println!("Copying entire directory...");
            copy_dir_recursively(&self.context_dir, &self.artifact_dir)?;
        } else {
            println!("COPY instruction(s) found in Formfile...");
            for (from, to) in copy_instructions {
                println!("COPY {from:?} to {to:?}...");
                let to = to.as_path()
                    .strip_prefix("./")
                    .or_else(|_| {
                        to.as_path()
                            .strip_prefix("/")
                            .or_else(|_| {
                                Ok::<&Path, Box<dyn std::error::Error>>(to.as_path())
                            })
                    })?.to_path_buf();
                let source = self.context_dir.join(from);
                let dest = self.artifact_dir.join(from);

                if source.is_dir() {
                    println!("{from:?} is a directory, copying recursively to {to:?}...");
                    fs::create_dir_all(&dest)?;
                    copy_dir_recursively(&source, &dest)?;
                } else {
                    if let Some(parent) = dest.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    println!("{from:?} is a file, copying to {to:?}...");
                    fs::copy(&source, &dest)?;
                }
            }

        }

        let tarball_path = self.artifact_dir.with_extension("tar.gz");
        let tarfile = fs::File::create(&tarball_path)?;
        let encoder = GzEncoder::new(tarfile, Compression::default());
        let mut archive = Builder::new(encoder);
        archive.append_dir_all(".", &self.artifact_dir)?;
        archive.finish()?;

        Ok(tarball_path)
    }

    pub fn cleanup(&self) -> io::Result<()> {
        fs::remove_dir_all(&self.artifact_dir)
    }
}
