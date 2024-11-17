use audiotags::Tag;
use clap::Parser;
use pathdiff::diff_paths;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Tags {
    artist: String,
    album: String,
    title: String,
}
impl Tags {
    fn remove_slashes(&mut self) {
        self.artist = self.artist.replace("/", "-");
        self.album = self.album.replace("/", "-");
        self.title = self.title.replace("/", "-");
    }
    fn remove_null_bytes(&mut self) {
        self.artist = self.artist.replace("\0", "");
        self.album = self.album.replace("\0", "");
        self.title = self.title.replace("\0", "");
    }
}

struct MusicFile {
    base_path: PathBuf,
    relative_path: PathBuf,
}
impl MusicFile {
    fn new(base: &PathBuf, relative: &PathBuf) -> Option<MusicFile> {
        let ret = MusicFile {
            base_path: base.clone(),
            relative_path: relative.clone(),
        };
        if ret.tag_available() {
            return Some(ret);
        }
        return None;
    }

    fn tag_available(&self) -> bool {
        let mut full_path = self.base_path.clone();
        full_path.push(&self.relative_path);
        let tag = Tag::new().read_from_path(full_path);
        return tag.is_ok();
    }

    fn compose_tags_from_path(&self) -> Tags {
        let mut ret = Tags {
            title: String::new(),
            album: String::new(),
            artist: String::new(),
        };
        let stem = self.relative_path.file_stem();
        if let Some(title) = stem {
            ret.title = title.to_str().unwrap().to_string();
        }
        if let Some(album_path) = self.relative_path.parent() {
            if let Some(album_dir) = album_path.file_name() {
                ret.album = album_dir.to_str().unwrap().to_string();
            }

            if let Some(artist_path) = album_path.parent() {
                if let Some(artist_dir) = artist_path.file_name() {
                    ret.artist = artist_dir.to_str().unwrap().to_string();
                }
            }
        }
        ret.remove_slashes();
        return ret;
    }

    fn compose_path_from_tags(&self, input_tags: &Tags) -> PathBuf {
        let tags = {
            let mut t = input_tags.clone();
            t.remove_null_bytes();
            t
        };
        let ext = self.relative_path.extension().unwrap().to_str().unwrap();
        let mut ret = PathBuf::new();
        ret.push(&tags.artist);
        ret.push(&tags.album);
        ret.push(tags.title.clone() + "." + ext);
        return ret;
    }

    fn tags(&self) -> Tags {
        let mut full_path = self.base_path.clone();
        full_path.push(&self.relative_path);
        let tag = Tag::new().read_from_path(full_path).unwrap();
        let mut ret = Tags {
            title: String::new(),
            album: String::new(),
            artist: String::new(),
        };
        if tag.title().is_some() {
            ret.title = tag.title().unwrap().to_string();
        }

        if tag.album_title().is_some() {
            ret.album = tag.album_title().unwrap().to_string();
        }

        if tag.artist().is_some() {
            ret.artist = tag.artist().unwrap().to_string();
        }
        ret.remove_slashes();
        return ret;
    }

    fn tags_match(&self) -> bool {
        let tags = self.compose_tags_from_path();
        return tags == self.tags();
    }

    fn paths_match(&self) -> bool {
        let real_path = self.relative_path.clone();
        let real_path_str = real_path.to_str();
        if real_path_str.is_none() {
            return false;
        }
        let path_from_tags = self.compose_path_from_tags(&self.tags());
        let path_from_tags_str = path_from_tags.to_str();
        if path_from_tags_str.is_none() {
            return false;
        }
        return Some(real_path_str) == Some(path_from_tags_str);
    }

    fn set_tags(&mut self, tags: &Tags) {
        let mut full_path = self.base_path.clone();
        full_path.push(&self.relative_path);
        let mut tag = Tag::new().read_from_path(full_path).unwrap();
        tag.set_title(&tags.title.as_str());
        tag.set_album_title(&tags.album.as_str());
        tag.set_artist(&tags.artist.as_str());
        let mut path = self.base_path.clone();
        path.push(self.relative_path.clone());
        tag.write_to_path(path.to_str().unwrap())
            .expect(format!("Fail to save to {:?}", path).as_str());
    }
}

enum MetaInfoSource {
    Filesystem,
    Tags,
}
struct FileSorter {
    base_path: PathBuf,
    metainfo_source: MetaInfoSource,
    dry_run: bool,
}
impl FileSorter {
    fn new(base: &PathBuf, metainfo_source: MetaInfoSource, dry_run: bool) -> Self {
        FileSorter {
            base_path: base.clone(),
            metainfo_source,
            dry_run,
        }
    }

    fn sort(&self) {
        for entry in WalkDir::new(self.base_path.clone()) {
            let absolute_path = entry.unwrap().path().to_path_buf();
            if absolute_path.is_dir() {
                print!("d");
                continue;
            }
            let relative_path =
                diff_paths(&absolute_path, &self.base_path).expect("Can't create relavie path");

            let music_file = MusicFile::new(&self.base_path, &relative_path.to_path_buf());
            if music_file.is_none() {
                print!("n");
                continue;
            }
            let mut music_file = music_file.unwrap();
            // comparing on potential result
            match self.metainfo_source {
                MetaInfoSource::Tags => {
                    if music_file.paths_match() {
                        print!(".");
                        continue;
                    }
                }
                MetaInfoSource::Filesystem => {
                    if music_file.tags_match() {
                        print!(".");
                        continue;
                    }
                }
            }
            println!("");
            let fs_tags = music_file.compose_tags_from_path();
            println!("Real path: {}", relative_path.display());
            println!(
                "Path from tags: {:?}",
                music_file
                    .compose_path_from_tags(&music_file.tags())
                    .as_path()
                    .as_os_str()
            );
            println!("Real Tags: {:?}", music_file.tags());
            println!("Tags from FS: {:?}", &fs_tags);
            if self.dry_run {
                continue;
            }
            match self.metainfo_source {
                MetaInfoSource::Filesystem => {
                    let tags = music_file.compose_tags_from_path();
                    music_file.set_tags(&tags);
                    println!("Modified tags for {:?}", &absolute_path)
                }
                MetaInfoSource::Tags => {
                    self.copy_to_tag_based_directory(music_file);
                }
            }
        }
        println!("");
    }

    fn copy_to_tag_based_directory(&self, file: MusicFile) {
        let mut source_full_path = self.base_path.to_path_buf();
        source_full_path.push(&file.relative_path);

        let mut dest_full_path = self.base_path.to_path_buf();
        dest_full_path.push(file.compose_path_from_tags(&file.tags()));

        println!("Moving from {:?} to {:?}", source_full_path, dest_full_path);
        let _ = fs::create_dir_all(dest_full_path.parent().unwrap());
        let move_res = fs::rename(&source_full_path, &dest_full_path);
        if let Err(err) = move_res {
            println!("Moving to {:?} failed, err: {}", dest_full_path, err);
            return;
        }
        println!("Done");
    }

    fn get_all_lossless(&self) -> Vec<PathBuf> {
        let mut res = Vec::<PathBuf>::new();
        for entry in WalkDir::new(self.base_path.clone()) {
            let absolute_path = entry.unwrap().path().to_path_buf();
            if absolute_path.is_dir() {
                continue;
            }
            if let Some(ext) = absolute_path.extension() {
                if ext == "flac" || ext == "m4a" {
                    res.push(absolute_path);
                }
            }
        }
        return res;
    }

    fn convert_all_flac_to_mp3(&self) {
        let limit = 18;

        let vec = self.get_all_lossless();
        let mut q = VecDeque::<std::thread::JoinHandle<()>>::new();
        for path in vec.iter() {
            if q.len() >= limit {
                let _ = q.pop_front().unwrap().join();
            }
            let p = path.to_path_buf();
            let handle = std::thread::spawn(move || {
                FileSorter::convert_to_mp3(&p);
            });
            q.push_back(handle);
        }

        while q.len() != 0 {
            let _ = q.pop_front().unwrap().join();
        }
    }

    fn convert_to_mp3(full_path: &PathBuf) {
        let mut output_path = full_path.clone();
        output_path.set_extension("mp3");
        println!("Converting {:?} -> {:?}", &full_path, &output_path);
        let res = Command::new("ffmpeg")
            .arg("-i")
            .arg(&full_path)
            .arg("-ab")
            .arg("320k")
            .arg(&output_path)
            .output();

        match res {
            Ok(_) => {
                let _ = std::fs::remove_file(&full_path).inspect_err(|e| {
                    println!("Failed to remove {}", e);
                });
            }
            Err(er) => {
                println!(
                    "Failed to convert {:?} -> {:?}. Err: {}",
                    &full_path, &output_path, er
                );
            }
        }
    }
}

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

    /// Move files to directories based on their tags
    #[arg(long, default_value_t = false)]
    source_from_tags: bool,
}

fn main() {
    let args = Args::parse();
    let metainfo_source = if args.source_from_tags {
        MetaInfoSource::Tags
    } else {
        MetaInfoSource::Filesystem
    };

    let base_path = std::env::current_dir().unwrap();
    let fs = FileSorter::new(&base_path, metainfo_source, args.dry_run);
    fs.sort();
    remove_non_music_direcotories(&base_path);
    fs.convert_all_flac_to_mp3();
}
