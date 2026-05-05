use futures::future::try_join_all;
use reqwest::{Client, Url, header::AUTHORIZATION};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt::Debug};

#[derive(Debug, Serialize, Deserialize)]
pub struct WriterInfo {
    pub id: u64,
    pub names: Vec<String>,
}

pub struct Genius {
    pub bearer: String,
    pub base: Url,
}

type DynError = Box<dyn Error + Sync + Send>;

impl Genius {
    pub fn new(access_token: &str) -> Self {
        Self {
            bearer: format!("Bearer {access_token}"),
            base: Url::parse("https://api.genius.com").unwrap(),
        }
    }

    pub async fn artist_songs(&self, id: u64) -> Result<Vec<ArtistSong>, DynError> {
        let client = Client::new();

        let href = self.base.join(&format!("artists/{id}/songs")).unwrap();
        let res = client
            .get(href)
            .header(AUTHORIZATION, &self.bearer)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_msg = res.text().await?;
            return Err(err_msg.into());
        }

        let x: GeniusRes<ArtistSongInner> = res.json().await?;
        Ok(x.response.songs)
    }

    pub async fn search(&self, key: &str) -> Result<Vec<SongShell>, DynError> {
        let client = Client::new();

        let href = self.base.join("search").unwrap();
        let query = vec![("q", key)];
        let res = client
            .get(href)
            .header(AUTHORIZATION, &self.bearer)
            .query(&query)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_msg = res.text().await?;
            return Err(err_msg.into());
        }

        let x: GeniusRes<SearchInner> = res.json().await?;
        Ok(x.response.hits.into_iter().map(|x| x.result).collect())
    }

    pub async fn artist(&self, id: u64) -> Result<Artist, DynError> {
        let client = Client::new();

        let href = self.base.join(&format!("artists/{id}")).unwrap();
        let res = client
            .get(href)
            .header(AUTHORIZATION, &self.bearer)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_msg = res.text().await?;
            return Err(err_msg.into());
        }

        let x: GeniusRes<ArtistInner> = res.json().await?;
        Ok(x.response.artist)
    }

    pub async fn song(&self, id: u64) -> Result<Song, DynError> {
        let client = Client::new();

        let href = self.base.join(&format!("songs/{id}")).unwrap();
        let res = client
            .get(href)
            .header(AUTHORIZATION, &self.bearer)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_msg = res.text().await?;
            return Err(err_msg.into());
        }

        let x: GeniusRes<SongInner> = res.json().await?;
        Ok(x.response.song)
    }

    pub async fn track_credits(&self, title: &str) -> Result<Vec<WriterInfo>, DynError> {
        let tracks = self.search(title).await?;
        let Some(top_track) = tracks.first() else {
            return Err("no tracks found".into());
        };
        let track_info = self.song(top_track.id).await?;

        let writers = try_join_all(track_info.writer_artists.into_iter().map(|w| async move {
            let w_info = self.artist(w.id).await?;
            let mut names = w_info.alternate_names.clone();
            names.push(w_info.name);
            Ok::<WriterInfo, DynError>(WriterInfo { id: w.id, names })
        }))
        .await?;

        Ok(writers)
    }
}

#[derive(Debug, Deserialize)]
struct GeniusRes<T>
where
    T: Debug,
{
    pub response: T,
}

#[derive(Debug, Deserialize)]
struct SongInner {
    pub song: Song,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Song {
    pub apple_music_id: String,
    pub artist_names: String,
    pub writer_artists: Vec<Writer>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Writer {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct SearchInner {
    pub hits: Vec<Search>,
}

#[derive(Debug, Deserialize)]
struct Search {
    // #[serde(rename = "type")]
    // pub res_type: String,
    pub result: SongShell,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SongShell {
    pub id: u64,
    pub title: String,
}

#[derive(Debug, Deserialize)]
struct ArtistInner {
    pub artist: Artist,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Artist {
    pub alternate_names: Vec<String>,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct ArtistSongInner {
    pub songs: Vec<ArtistSong>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ArtistSong {
    pub id: u64,
    pub artist_names: String,
    pub title: String,
}
