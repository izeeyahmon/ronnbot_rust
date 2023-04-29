use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::StatusCode;

use serde::{Deserialize, Serialize};
use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{
    CommandDataOption, CommandDataOptionValue,
};
use std::env::{self};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct Unauthorized {
    message: String,
}
impl fmt::Display for Unauthorized {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl Error for Unauthorized {}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub collections: Vec<Collection>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub collection_id: String,
    pub name: String,
    pub contract: String,
    pub image: Option<String>,
    pub all_time_volume: f64,
    pub floor_ask_price: Option<FloorAskPrice>,
    pub opensea_verification_status: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorAskPrice {
    pub currency: Currency,
    pub amount: Amount,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Currency {
    pub contract: String,
    pub name: String,
    pub symbol: String,
    pub decimals: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    pub raw: String,
    pub decimal: f64,
    pub usd: f64,
    pub native: f64,
}
pub async fn run(options: &[CommandDataOption]) -> String {
    let option = options
        .get(0)
        .expect("Expected Collection Name")
        .resolved
        .as_ref()
        .expect("Expected Collection Object");
    let verbose_flag;
    let verbose = options.get(1).is_none();

    if verbose {
        verbose_flag = false;
    } else {
        verbose_flag = options
            .get(1)
            .unwrap()
            .value
            .as_ref()
            .unwrap()
            .as_bool()
            .unwrap();
    }

    let empty_currency = Currency {
        contract: String::from("null"),
        name: String::from("null"),
        symbol: String::from("null"),
        decimals: 0,
    };
    let empty_amount = Amount {
        raw: String::from("null"),
        decimal: 0.0,
        usd: 0.0,
        native: 0.0,
    };
    let empty_floor = FloorAskPrice {
        currency: empty_currency,
        amount: empty_amount,
    };

    if let CommandDataOptionValue::String(collection) = option {
        let api_result = call_api(collection).await;
        let mut aggregated_output = String::new();
        match api_result {
            Ok(api_output) => {
                let result_length = *&api_output.collections.len() as u32;
                if result_length == 0 {
                    return format!("There is no collection found for the name {} ", collection);
                }
                if verbose_flag {
                    for project in &api_output.collections {
                        let proj_name = &project.name;
                        let floor_price = &project
                            .floor_ask_price
                            .as_ref()
                            .unwrap_or(&empty_floor)
                            .amount
                            .decimal;
                        let temp_string = format!(
                            "The floor price for [{}] is [{}] \n",
                            proj_name, floor_price
                        );
                        println!("test {} is {}", proj_name, temp_string);
                        aggregated_output.push_str(&temp_string);
                    }
                    aggregated_output
                } else {
                    let floor_price = &api_output.collections[0]
                        .floor_ask_price
                        .as_ref()
                        .unwrap_or(&empty_floor)
                        .amount
                        .decimal;
                    let project_name = &api_output.collections[0].name;
                    format!(
                        "The floor price for [{}] is [{}]",
                        project_name, floor_price
                    )
                }
            }
            Err(_) => format!("Something went wrong contact izee"),
        }
    } else {
        "Please Provide a collection name".to_string()
    }
}
//test
pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("floorprice")
        .description("Get a Floor Price of a Collection")
        .create_option(|option| {
            option
                .name("project")
                .description("String Name of the Collection")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("verbose")
                .description("If we pull display all the verbose stuff")
                .kind(CommandOptionType::Boolean)
                .required(false)
        })
}

pub async fn call_api(nft_collection: &String) -> Result<Root, Box<dyn Error>> {
    let base_url = "https://api.reservoir.tools/search/collections/v2?name=";

    let url = format!("{}{}", base_url, nft_collection);
    let mut headers = HeaderMap::new();
    {
        let api_key = match env::var("RESERVOIR_API_KEY") {
            Ok(val) => val,
            Err(_) => {
                return Err(Box::new(Unauthorized {
                    message: String::from("test"),
                }))
            }
        };
        headers.insert(
            HeaderName::from_static("x-api-key"),
            HeaderValue::from_str(api_key.as_str()).unwrap(),
        );
    }

    headers.insert(
        HeaderName::from_static("accept"),
        HeaderValue::from_static("*/*"),
    );
    let client = reqwest::Client::new();
    let response = client.get(url).headers(headers).send().await.unwrap();

    match response.status() {
        StatusCode::OK => {
            // on success, parse our JSON to an APIResponse
            match response.json::<Root>().await {
                Ok(parsed) => Ok(parsed),
                Err(_e) => Err(Box::new(Unauthorized {
                    message: String::from("Failed to parse"),
                })),
            }
        }

        other => {
            panic!("Uh oh! Something unexpected happened: {:?}", other)
        }
    }
}
