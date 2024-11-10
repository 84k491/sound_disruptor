use audiotags::Tag;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct Tags {
    title: String,
    album: String,
    artist: String,
}

struct MusicFile {
    relative_path: PathBuf,
}
impl MusicFile {
    fn new(path: &str) -> MusicFile {
        let ret = MusicFile {
            relative_path: Path::new(path).to_path_buf(),
        };
        return ret;
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
        return ret;
    }
    fn tags(&self) -> Tags {
        let tag = Tag::new().read_from_path(&self.relative_path).unwrap();
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
        return ret;
    }
}

fn main() {
    let music_file = MusicFile::new("/home/bakar/tmp/mu_conv/some_artist/some_album/track.flac");

    println!("{:?}", music_file.compose_tags_from_path());

    println!("{:?}", music_file.tags());
}
