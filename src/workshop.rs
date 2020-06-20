use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct PublishedFileDetails {
	pub publishedfileid: String,
	pub title: Option<String>,
    pub description: Option<String>,
    pub file_url: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct PublishedFileDetailsList {
    pub result: u8,
	pub resultcount: usize,
	pub publishedfiledetails: Vec<PublishedFileDetails>,
}

#[derive(Deserialize, Debug)]
pub struct PublishedFileDetailsResponse {
	pub response: PublishedFileDetailsList
}

pub async fn published_file_details(workshop_id: u64) -> Result<Option<PublishedFileDetails>, Box<dyn std::error::Error>> {
    let url = "https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1/";

    let client = reqwest::Client::new();
    let params = [("format", "json"), ("itemcount", "1"), ("publishedfileids[0]", &format!("{}", workshop_id))];
    let res = client.post(url)
        .form(&params)
        .send()
        .await?
        .json::<PublishedFileDetailsResponse>()
        .await?;
    
    match res {
        PublishedFileDetailsResponse {
            response: PublishedFileDetailsList {
                publishedfiledetails: mut details,
                ..
            }
        } if details.len() > 0 => Ok(Some(details.remove(0))),
        _ => Ok(None)
    }
}