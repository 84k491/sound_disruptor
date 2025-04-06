pub mod file_sorter {
    use crate::music_file::music_file::MusicFile;
    use pathdiff::diff_paths;
    use std::{collections::VecDeque, fs, path::PathBuf, process::Command};
    use walkdir::WalkDir;

    pub enum ActionOnFile {
        ModifyTags,
        MoveFiles,
    }

    pub struct FileSorter {
        pub base_path: PathBuf,
        pub metainfo_source: ActionOnFile,
        pub dry_run: bool,
    }

    impl FileSorter {
        pub fn new(base: &PathBuf, metainfo_source: ActionOnFile, dry_run: bool) -> Self {
            FileSorter {
                base_path: base.clone(),
                metainfo_source,
                dry_run,
            }
        }

        pub fn for_each_music_file<F: FnMut(&FileSorter, MusicFile)>(&self, mut f: F) {
            for entry in WalkDir::new(self.base_path.clone()) {
                let absolute_path = entry.unwrap().path().to_path_buf();
                if absolute_path.is_dir() {
                    continue;
                }
                let relative_path = diff_paths(&absolute_path, &self.base_path)
                    .expect("Can't create relative path");
                let music_file = MusicFile::new(&self.base_path, &relative_path.to_path_buf());
                if music_file.is_none() {
                    continue;
                }
                let music_file = music_file.unwrap();
                f(self, music_file);
            }
        }

        pub fn move_file(&self, music_file: MusicFile) {
            // comparing on potential result
            if music_file.paths_match() {
                print!(".");
                return;
            }

            println!("");
            println!("Real path: {}", music_file.relative_path.display());
            println!(
                "Path from tags: {:?}",
                music_file
                    .compose_path_from_tags(&music_file.tags())
                    .as_path()
                    .as_os_str()
            );

            if self.dry_run {
                return;
            }

            self.copy_to_tag_based_directory(music_file);
            println!("");
        }

        pub fn modify_tags(&self, music_file: MusicFile) {
            // comparing on potential result
            if music_file.tags_match()
                && music_file.tags().verify_artists()
                && music_file.tags().verify_track_number()
            {
                print!(".");
                return;
            }

            println!("");
            let mut fs_tags = music_file.compose_tags_from_path();
            println!("Path: {}", music_file.relative_path.display());
            println!("Old Tags: {:?}", music_file.tags());
            println!("New tags: {:?}", &fs_tags);
            if self.dry_run {
                return;
            }

            fs_tags.fix_track_number();
            let mut music_file = music_file;

            music_file.set_tags(&fs_tags);
            println!("Modified tags for {:?}.", &music_file.relative_path)
        }

        pub fn run(&self) {
            match self.metainfo_source {
                ActionOnFile::MoveFiles => {
                    self.for_each_music_file(|fs, mf| fs.move_file(mf));
                }
                ActionOnFile::ModifyTags => {
                    self.for_each_music_file(|fs, mf| fs.modify_tags(mf));
                }
            }
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

        pub fn convert_all_flac_to_mp3(&self) {
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
}
