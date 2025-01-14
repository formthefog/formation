use std::{path::{Path, PathBuf}, fs, io};
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder;
use crate::image_builder::copy_dir_recursively;

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
        copy_instructions: &[PathBuf],
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        if copy_instructions.is_empty() {
            copy_dir_recursively(&self.context_dir, &self.artifact_dir)?;
        } else {
            for path in copy_instructions {
                let source = self.context_dir.join(path);
                let dest = self.artifact_dir.join(path);

                if source.is_dir() {
                    fs::create_dir_all(&dest)?;
                    copy_dir_recursively(&source, &dest)?;
                } else {
                    if let Some(parent) = dest.parent() {
                        fs::create_dir_all(parent)?;
                    }
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
