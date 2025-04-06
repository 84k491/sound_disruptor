use crate::file_sorter::file_sorter::FileSorter;
use crate::file_sorter::file_sorter::ActionOnFile;
use audiotags::Tag;
use clap::Parser;
use std::path::PathBuf;

mod music_file;
mod tags;
mod file_sorter;

fn remove_non_music_direcotories(path: &PathBuf) -> bool {
    let res = std::fs::read_dir(&path);
    if let Err(err) = res {
        println!("Can't read directory: {:?}, err: {}", &path, err);
        return false;
    }

    let mut all_contents_removed = true;
    let entries = res.unwrap();
    for e in entries {
        if let Err(err) = e {
            println!("Can't read dir entry: {}", err);
            continue;
        }
        let entry = e.unwrap();
        let metadata_res = std::fs::metadata(entry.path());
        if let Err(err) = metadata_res {
            println!("Can't get metadata: {:?}, err: {}", entry.path(), err);
            continue;
        }
        let metadata = metadata_res.unwrap();
        if metadata.is_dir() {
            let removed = remove_non_music_direcotories(&entry.path());
            all_contents_removed = all_contents_removed & removed;
            continue;
        }

        let tags = Tag::new().read_from_path(entry.path());
        let has_tags = tags.is_ok();
        if has_tags {
            all_contents_removed = false;
            continue;
        }

        println!("Removing file: {:?}", entry.path());
        if let Err(err) = std::fs::remove_file(entry.path()) {
            println!("Can't remove a file: {:?}, err: {}", entry.path(), err);
        }
    }

    if !all_contents_removed {
        return false;
    }

    println!("Removing directory: {:?}", &path);
    if let Err(err) = std::fs::remove_dir(&path) {
        println!("Can't remove a directory: {:?}, err: {}", &path, err);
        return false;
    }

    return true;
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Just scan, print output and do nothing
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Move files to directories based on their tags. Modify tags without this flag
    #[arg(long, default_value_t = false)]
    move_files: bool,
}

fn main() {
    let args = Args::parse();
    let metainfo_source = if args.move_files {
        ActionOnFile::MoveFiles
    } else {
        ActionOnFile::ModifyTags
    };

    let base_path = std::env::current_dir().unwrap();
    let fs = FileSorter::new(&base_path, metainfo_source, args.dry_run);
    fs.run();
    remove_non_music_direcotories(&base_path);
    fs.convert_all_flac_to_mp3();
}
