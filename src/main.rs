use audiotags::Tag;
use clap::Parser;
use pathdiff::diff_paths;
use remove_empty_subdirs::remove_empty_subdirs;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, PartialEq, Eq)]
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

    fn compose_path_from_tags(&self, tags: &Tags) -> PathBuf {
        let mut ret = PathBuf::new();
        ret.push(&tags.artist);
        ret.push(&tags.album);
        ret.push(&tags.title);
        let ext = self.relative_path.extension().unwrap();
        ret.set_extension(ext);
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

    fn tags_match_filesystem(&self) -> bool {
        let tags = self.compose_tags_from_path();
        return tags == self.tags();
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
            if music_file.tags_match_filesystem() {
                print!(".");
                continue;
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
                    println!("Modified tags for {:?}", absolute_path)
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
        if !move_res.is_ok() {
            println!("Moving to {:?} failed", dest_full_path);
            return;
        }
        println!("Done");
    }
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
    let _ = remove_empty_subdirs(&base_path);
}
