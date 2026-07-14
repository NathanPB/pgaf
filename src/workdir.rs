use std::error::Error;
use std::fs::create_dir_all;
use std::path::PathBuf;

pub fn make_workdir(
    workdir: &Option<PathBuf>,
    keep: &Option<bool>,
    overwrite: bool,
) -> Result<(PathBuf, bool), Box<dyn Error>> {
    let keep = workdir.is_some() || keep.unwrap_or(false);

    let new_workdir = match workdir {
        Some(workdir) => create_dir_all(workdir).map(|_| workdir.to_path_buf())?,
        None => tempfile::Builder::new()
            .prefix("pgaf-workdir")
            .keep(keep)
            .tempdir()?
            .into_path(),
    };

    if overwrite {
        for entry in new_workdir.read_dir()? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                std::fs::remove_dir_all(path)?;
            } else {
                std::fs::remove_file(path)?;
            }
        }
    }

    Ok((new_workdir, !keep))
}
