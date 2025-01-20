pub mod music_file {
    use crate::tags::tags::Tags;
    use crate::Tag;
    use std::path::PathBuf;

    pub struct MusicFile {
        pub base_path: PathBuf,
        pub relative_path: PathBuf,
    }
    impl MusicFile {
        pub fn new(base: &PathBuf, relative: &PathBuf) -> Option<MusicFile> {
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

        pub fn compose_tags_from_path(&self) -> Tags {
            let mut ret = Tags {
                title: String::new(),
                album: String::new(),
                album_artist: String::new(),
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
            ret.remove_invalid_symbols();
            return ret;
        }

        pub fn compose_path_from_tags(&self, input_tags: &Tags) -> PathBuf {
            let tags = {
                let mut t = input_tags.clone();
                t.remove_null_bytes();
                t.remove_invalid_symbols();
                t
            };
            let ext = self.relative_path.extension().unwrap().to_str().unwrap();
            let mut ret = PathBuf::new();
            ret.push(&tags.artist);
            // it's a path, no need to push album artist
            ret.push(&tags.album);
            ret.push(tags.title.clone() + "." + ext);
            return ret;
        }

        pub fn tags(&self) -> Tags {
            let mut full_path = self.base_path.clone();
            full_path.push(&self.relative_path);
            let tag = Tag::new().read_from_path(&full_path).unwrap();
            let mut ret = Tags {
                title: String::new(),
                album: String::new(),
                album_artist: String::new(),
                artist: String::new(),
            };

            if let Some(title) = tag.title() {
                ret.title = title.to_string();
            }

            if let Some(album) = tag.album_title() {
                ret.album = album.to_string();
            }

            if let Some(artist) = tag.artist() {
                ret.artist = artist.to_string();
            }

            if let Some(album_artist) = tag.album_artist() {
                ret.album_artist = album_artist.to_string();
            }

            ret.remove_slashes();
            ret.remove_invalid_symbols();
            return ret;
        }

        pub fn tags_match(&self) -> bool {
            let tags = self.compose_tags_from_path();
            return tags == self.tags();
        }

        pub fn paths_match(&self) -> bool {
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

        pub fn set_tags(&mut self, tags: &Tags) {
            let mut full_path = self.base_path.clone();
            full_path.push(&self.relative_path);
            let mut tag = Tag::new().read_from_path(full_path).unwrap();
            tag.remove_album_artist();
            tag.set_title(&tags.title.as_str());
            tag.set_album_title(&tags.album.as_str());
            tag.set_artist(&tags.artist.as_str());
            let mut path = self.base_path.clone();
            path.push(self.relative_path.clone());
            tag.write_to_path(path.to_str().unwrap())
                .expect(format!("Fail to save to {:?}", path).as_str());
        }
    }
}
