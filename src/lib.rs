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
    client: Client,
}

type DynError = Box<dyn Error + Sync + Send>;

impl Genius {
    pub fn new(access_token: &str) -> Self {
        Self {
            bearer: format!("Bearer {access_token}"),
            base: Url::parse("https://api.genius.com").unwrap(),
            client: Client::new(),
        }
    }

    async fn find_credit(&self, res: &SongShell, name: &str) -> Result<Option<u64>, DynError> {
        let song = self.song(res.id).await?;
        for party in song.writer_artists.into_iter().chain(song.producer_artists) {
            // TODO: make spelling tolerance
            // TODO: if name is close (medium confidence, search for alternate names)
            if party.name.eq_ignore_ascii_case(name) {
                return Ok(Some(party.id));
            }
        }
        Ok(None)
    }

    pub async fn identify_artist_id(
        &self,
        artist_name: &str,
        titles: &[&str],
    ) -> Result<Option<u64>, DynError> {
        for title in titles {
            let results = self.search(title).await?;
            for res in &results {
                let Some(id) = self.find_credit(res, artist_name).await? else {
                    continue;
                };
                return Ok(Some(id));
            }
        }
        Ok(None)
    }

    pub async fn artist_songs(&self, id: u64) -> Result<Vec<ArtistSong>, DynError> {
        let href = self.base.join(&format!("artists/{id}/songs")).unwrap();
        let res = self
            .client
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
        let href = self.base.join("search").unwrap();
        let query = vec![("q", key)];
        let res = self
            .client
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
        let href = self.base.join(&format!("artists/{id}")).unwrap();
        let res = self
            .client
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
        let href = self.base.join(&format!("songs/{id}")).unwrap();
        let res = self
            .client
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

    pub async fn track_credits(&self, id: u64) -> Result<Vec<WriterInfo>, DynError> {
        let track_info = self.song(id).await?;

        let writers = try_join_all(track_info.writer_artists.into_iter().map(|w| async move {
            let mut writer = self.artist(w.id).await?;
            writer.alternate_names.push(writer.name);
            writer.alternate_names.iter_mut().for_each(|x| {
                *x = x
                    .chars()
                    .map(|c| if c.is_ascii() { c } else { ' ' })
                    .collect::<String>()
                    .split_whitespace()
                    .collect::<Vec<&str>>()
                    .join(" ")
            });
            writer.alternate_names.sort();
            writer
                .alternate_names
                .dedup_by(|x, y| x.to_lowercase() == y.to_lowercase());

            Ok::<WriterInfo, DynError>(WriterInfo {
                id: w.id,
                names: writer.alternate_names,
            })
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
    pub producer_artists: Vec<Writer>,
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
    pub image_url: Option<String>,
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
