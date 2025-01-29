use std::{collections::HashSet, path::PathBuf, sync::Mutex};

const YTM_WATCH: &str = "https://music.youtube.com/watch?v=";

use flume::Sender;
use log::{error, trace};
use once_cell::sync::Lazy;
// use rusty_ytdl::{
//     DownloadOptions, Video, VideoError, VideoOptions, VideoQuality, VideoSearchOptions,
// };
use yt_dlp::fetcher::deps::Libraries;
use yt_dlp::{error::Error as VideoError, Youtube};
use ytpapi2::YoutubeMusicVideoRef;

use crate::{
    consts::CACHE_DIR,
    run_service,
    structures::{app_status::MusicDownloadStatus, sound_action::SoundAction},
    systems::download::HANDLES,
};

// fn new_video_with_id(id: &str) -> Result<Video, VideoError> {
// let search_options = VideoSearchOptions::Custom(Arc::new(|format| {
//     format.has_audio && !format.has_video && format.mime_type.container == "mp4"
// }));
// let video_options = VideoOptions {
//     quality: VideoQuality::Custom(
//         search_options.clone(),
//         Arc::new(|x, y| x.audio_bitrate.cmp(&y.audio_bitrate)),
//     ),
//     filter: search_options,
//     download_options: DownloadOptions {
//         dl_chunk_size: Some(1024 * 100_u64),
//     },
//     ..Default::default()
// };

// Video::new_with_options(id, video_options)
// }

// pub async fn download<P: AsRef<std::path::Path>>(
//     video: &Video<'_>,
//     path: P,
//     sender: Sender<SoundAction>,
// ) -> Result<(), VideoError> {
//     use std::io::Write;
//     let stream = video.stream().await?;

//     let length = stream.content_length();

//     let mut file =
//         std::fs::File::create(&path).map_err(|e| VideoError::DownloadError(e.to_string()))?;

//     let mut total = 0;
//     while let Some(chunk) = stream.chunk().await? {
//         total += chunk.len();

//         sender
//             .send(SoundAction::VideoStatusUpdate(
//                 video.get_video_id(),
//                 MusicDownloadStatus::Downloading((total as f64 / length as f64 * 100.0) as usize),
//             ))
//             .unwrap();

//         file.write_all(&chunk)
//             .map_err(|e| VideoError::DownloadError(e.to_string()))?;
//     }

//     file.flush()
//         .map_err(|e| VideoError::DownloadError(e.to_string()))?;

//     if total != length || length == 0 {
//         std::fs::remove_file(path).map_err(|e| VideoError::DownloadError(e.to_string()))?;
//         return Err(VideoError::DownloadError(format!(
//             "Downloaded file is not the same size as the content length ({}/{})",
//             total, length
//         )));
//     }

//     Ok(())
// }

async fn handle_download(id: &str, sender: Sender<SoundAction>) -> Result<(), VideoError> {
    let idc = id.to_string();

    //let video = new_video_with_id(id)?;

    sender
        .send(SoundAction::VideoStatusUpdate(
            idc.clone(),
            MusicDownloadStatus::Downloading(0),
        ))
        .unwrap();
    //let file = CACHE_DIR.join("downloads").join(format!("{id}.mp4"));
    //download(&video, file, sender.clone()).await?;

    let file = format!("{id}.mp4");
    let libraries_dir = PathBuf::from(CACHE_DIR.join("lib"));
    let output_dir = PathBuf::from(CACHE_DIR.join("downloads"));

    let youtube = libraries_dir.join("yt-dlp");
    let ffmpeg = libraries_dir.join("ffmpeg");
    let url = [YTM_WATCH, id].join("");

    let libraries = Libraries::new(youtube, ffmpeg);
    let fetcher = Youtube::new(libraries, output_dir)?;

    match fetcher.fetch_video_infos(url).await {
        Ok(video) => {
            println!("Video title: {}", video.title);
        }
        Err(ef) => {
            println!("error: {:?}", ef);
            panic!("error : {:?}",ef);
        }
    }

    // let video = fetcher.fetch_video_infos(video).await?;
    // trace!("Video title: {}", video.title);
    // let audio_format = video.best_audio_format().unwrap();
    // fetcher.download_format(&audio_format, &file).await?;

    sender
        .send(SoundAction::VideoStatusUpdate(
            idc.clone(),
            MusicDownloadStatus::Downloading(100),
        ))
        .unwrap();

    // rustube::Video::from_id(Id::from_str(id)?.into_owned())
    //     .await?
    //     .streams()
    //     .iter()
    //     .filter(|stream| {
    //         stream.mime == "audio/mp4"
    //             && stream.includes_audio_track
    //             && !stream.includes_video_track
    //     })
    //     .max_by_key(|stream| stream.bitrate)
    //     .ok_or(Error::NoStreams)?
    //     .download_to_dir_with_callback(
    //         CACHE_DIR.join("downloads"),
    //         Callback::new().connect_on_progress_closure(move |progress| {
    //             let perc = progress
    //                 .content_length
    //                 .as_ref()
    //                 .map(|x| progress.current_chunk * 100 / *x as usize)
    //                 .unwrap_or(0);
    //             sender
    //                 .send(SoundAction::VideoStatusUpdate(
    //                     idc.clone(),
    //                     MusicDownloadStatus::Downloading(perc),
    //                 ))
    //                 .unwrap();
    //         }),
    //     )
    //     .await?;
    Ok(())
}

pub static IN_DOWNLOAD: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

// entry point of download task
pub async fn start_download(song: YoutubeMusicVideoRef, s: &Sender<SoundAction>) -> bool {
    {
        let mut downloads = IN_DOWNLOAD.lock().unwrap();
        if downloads.contains(&song.video_id) {
            return false;
        }
        downloads.insert(song.video_id.clone());
    }
    s.send(SoundAction::VideoStatusUpdate(
        song.video_id.clone(),
        MusicDownloadStatus::Downloading(1),
    ))
    .unwrap();
    let download_path_mp4 = CACHE_DIR.join(format!("downloads/{}.mp4", &song.video_id));
    let download_path_json = CACHE_DIR.join(format!("downloads/{}.json", &song.video_id));
    if download_path_json.exists() {
        s.send(SoundAction::VideoStatusUpdate(
            song.video_id.clone(),
            MusicDownloadStatus::Downloaded,
        ))
        .unwrap();
        return true;
    }
    if download_path_mp4.exists() {
        std::fs::remove_file(&download_path_mp4).unwrap();
    }
    match handle_download(&song.video_id, s.clone()).await {
        Ok(_) => {
            std::fs::write(download_path_json, serde_json::to_string(&song).unwrap()).unwrap();
            crate::append(song.clone());
            s.send(SoundAction::VideoStatusUpdate(
                song.video_id.clone(),
                MusicDownloadStatus::Downloaded,
            ))
            .unwrap();
            IN_DOWNLOAD.lock().unwrap().remove(&song.video_id);
            true
        }
        Err(e) => {
            if download_path_mp4.exists() {
                std::fs::remove_file(download_path_mp4).unwrap();
            }
            s.send(SoundAction::VideoStatusUpdate(
                song.video_id.clone(),
                MusicDownloadStatus::DownloadFailed,
            ))
            .unwrap();
            error!("Error downloading {}: {e}", song.video_id);
            false
        }
    }
}
pub fn start_task_unary(s: Sender<SoundAction>, song: YoutubeMusicVideoRef) {
    HANDLES.lock().unwrap().push(run_service(async move {
        start_download(song, &s).await;
    }));
}

// TODO write a test

#[tokio::test]
async fn video_download_test() {
    let ids = vec!["iFbNzVFgjCk"];
    for id in ids {
        let libraries_dir = PathBuf::from(CACHE_DIR.join("lib"));
        let output_dir = PathBuf::from(CACHE_DIR.join("downloads"));
    
        let youtube = libraries_dir.join("yt-dlp");
        let ffmpeg = libraries_dir.join("ffmpeg");
        let url = [YTM_WATCH, id].join("");
    
        let libraries = Libraries::new(youtube, ffmpeg);
        match Youtube::new(libraries, output_dir){
            Ok(fetcher) => {
                match fetcher.fetch_video_infos(url).await {
                    Ok(video) => {
                        println!("Video title: {}", video.title);
                    }
                    Err(ef) => {
                        println!("error: {:?}", ef);
                        panic!("error : {:?}",ef);
                    }
                }
            }
            Err(e) => {
                panic!("error : {:?}", e)
            }
        }
        // let audio_format = video.best_audio_format().unwrap();
        // fetcher.download_format(&audio_format, &file).await;
    }
}

// #[tokio::test]
// async fn video_download_test() {
//     let ids = vec!["iFbNzVFgjCk"];
//     for id in ids {
//         let video = Video::new(id).unwrap();
//         let stream = video.stream().await.unwrap();
//         let content_length = stream.content_length();
//         let mut total = 0;
//         while let Some(chunk) = stream.chunk().await.unwrap() {
//             total += chunk.len();
//         }
//         assert_eq!(total, content_length);
//     }
// }
