use std::env;

use dotenv::dotenv;
use genius_api::Genius;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let access_token = env::var("ACCESS_TOKEN").expect("missing access token");
    let genius = Genius::new(&access_token);
    // let writers = genius
    //     .track_credits("Peaches Justin Bieber")
    //     .await
    //     .unwrap();
    let tracks = genius.artist_songs(620527).await.unwrap();
    dbg!(tracks);
    // let artist_id = genius
    //     .identify_artist_id(
    //         "aidan rodriguez",
    //         &["Drunk Tank Marc E Bassy", "Sexy Villain Remi Wolf"],
    //     )
    //     .await
    //     .unwrap();
    // dbg!(artist_id);
}
