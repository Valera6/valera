use crate::requests::{Client, ClientSpecific};
use crate::types::*;
use polars::prelude::{df, DataFrame, NamedFrom};

pub enum Templates{
	BinancePerp,
	BinanceSpot,
	SomethingElseSayCoinmarketcap,
}
impl Templates {
	pub fn build(&self) -> Provider {
		match self {
			Self::BinancePerp => Provider::build(
				vec![
					ClientSpecific{ api_key: Some(std::env::var("BINANCE_MAIN_KEY").unwrap()), proxy: None },
				],
				1500,
				Some("https://fapi.binance.com/fapi/v1"),
				Box::new(|current_used: i32, r: &reqwest::Response| -> i32 {
					let header_value = r.headers().get("x-mbx-used-weight-1m").unwrap();
					match header_value.to_str() {
						Ok(used_str) => used_str.parse::<i32>().unwrap_or(current_used),
						Err(_) => {
							eprintln!("Error: failed to extract new used from reqwest::Response");
							current_used
						}
					}
				})
			),
			//Self::BinanceSpot => Provider::build(), // rate limit here is 6000, btw
			_ => panic!("Not implemented yet"),
		}
	}
}

pub struct Provider {	
	base_url: String,
	clients: Vec<Client>,
}
impl Provider {
	pub fn default() -> Self {
		todo!()
	}
	pub fn build<F>(clients: Vec<ClientSpecific>, rate_limit: i32, base_url: Option<&str>, calc_used: Box<F>) -> Self
	where
		F: Fn(i32, &reqwest::Response) -> i32 + Clone,
	{
		let base_url = match base_url {
			Some(s) => s.to_owned(),
			None => "".to_owned(),
		};
		let clients: Vec<Client> = clients.iter().map(|&client_specific| Client::build(client_specific, rate_limit, calc_used.clone())).collect();
		Provider{
			clients,
			base_url,
		}	
	}
}

//=============================================================================
impl Providers {
	pub fn trades_entry_into_row(&self, entry: &serde_json::Value) -> DataFrame {
		df!(
			"time_ms" => vec![entry.get("time").unwrap().as_i64().unwrap()],
			"price" => vec![entry.get("price").unwrap().as_str().unwrap().parse::<f64>().unwrap()],
			"qty" => vec![entry.get("quoteQty").unwrap().as_str().unwrap().parse::<f64>().unwrap()],
			"isBuyerMaker" => vec![entry.get("isBuyerMaker").unwrap().as_bool().unwrap()],
		)
		.unwrap()
	}
	pub fn convert_into_klines(&self, array_of_values: Vec<serde_json::Value>) -> Klines {
		match self {
			Providers::BinancePerp | Providers::BinanceSpot => {
				// these are the values that every array returned by /klines endpoint carries:
				//let columns = ["open_ms", "open", "high", "low", "close", "volume", "close_ms", "quote_asset_volume", "trades", "taker_buy_base", "taker_buy_quote", "ignore"];
				// let indeces = [6, 1, 2, 3, 4, 7, 8, 10]; // these are the ones we care about

				let mut close_ms: Vec<i64> = Vec::new();
				let mut open: Vec<f64> = Vec::new();
				let mut high: Vec<f64> = Vec::new();
				let mut low: Vec<f64> = Vec::new();
				let mut close: Vec<f64> = Vec::new();
				let mut volume: Vec<f64> = Vec::new();
				let mut trades: Vec<f64> = Vec::new();
				let mut taker_volume: Vec<f64> = Vec::new();
				for v in array_of_values.iter() {
					close_ms.push(v[6].as_i64().unwrap());
					open.push(v[1].as_str().unwrap().parse().unwrap());
					high.push(v[2].as_str().unwrap().parse().unwrap());
					low.push(v[3].as_str().unwrap().parse().unwrap());
					close.push(v[4].as_str().unwrap().parse().unwrap());
					volume.push(v[7].as_str().unwrap().parse().unwrap());
					trades.push(v[8].as_f64().unwrap());
					taker_volume.push(v[10].as_str().unwrap().parse().unwrap());
				}

				let df = df!(
					"close_ms" => close_ms,
					"open" => open,
					"high" => high,
					"low" => low,
					"close" => close,
					"volume" => volume,
					"trades" => trades,
					"taker_volume" => taker_volume,
				)
				.unwrap();
				let k: Klines = df.try_into().unwrap();
				k
			}
			_ => panic!("Conversion to klines for this Market is not supported yet"),
		}
	}
}
