use crate::bytecode_analyzer::get_most_similar_contracts;
use crate::*;
use std::time::Instant;
use std::{str::FromStr, time::Duration};
use tokio::net::TcpListener;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;
use tokio_tungstenite::{self, tungstenite::Message};
use tokio_tungstenite::{accept_async, WebSocketStream};
use walkdir::WalkDir;
use warp::{reply, Filter};

const HTTP_PORT: u16 = 9003;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MostSimilarContracts {
    pub address: Address,
    pub most_similar_contracts: Vec<String>,
}

pub async fn run_endpoint_handler() -> eyre::Result<()> {
    let cors = warp::cors()
        .allow_any_origin();
    let similar_contracts = serde_json::from_str::<Vec<ProtocolEventsFns>>(include_str!(
        "../../inputs/protocol_events_fns.json"
    ))
    .unwrap();
    let selectors =
        serde_json::from_str::<Selectors>(include_str!("../../inputs/selectors.json")).unwrap();
    let events = serde_json::from_str::<Events>(include_str!("../../inputs/events.json")).unwrap();

    let get_contract_bytecode = warp::path!("get_bytecode_for_address" / Address).map( move |address: Address| {
        // use address to find matching bytecode txt file within db/filtered_bytecodes/*
        let address_str = format!("{:?}", address);
        for entry in WalkDir::new("./db/filtered_bytecodes").into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()) {

            // check that the file name contains the address
            let entry_filename = entry.file_name().to_str().unwrap();
            if entry_filename.contains(&address_str) {
                let entry_path = entry.path();
                let entry_filename = entry.file_name().to_str().unwrap();
                // read the file and return the contents
                let bytecode_contents = std::fs::read_to_string(entry_path.clone()).unwrap();
                // convert bytecode_contents to Bytes type
                return Ok(reply::json(&bytecode_contents));
            } else {
                continue;
            }
        }
        return Ok(reply::json(&"No matches found"));
    });


    let get_similar_contracts =
        warp::path!("get_similar_contract_for_address" / Address).map(move |address: Address| {
            // use address to find matching bytecode txt file within db/filtered_bytecodes/*
            let address_str = format!("{:?}", address);
            for entry in WalkDir::new("./db/filtered_bytecodes").into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()) {

                // check that the file name contains the address
                let entry_filename = entry.file_name().to_str().unwrap();
                if entry_filename.contains(&address_str) {
                    let entry_path = entry.path();
                    let entry_filename = entry.file_name().to_str().unwrap();
                    // read the file and return the contents
                    let bytecode_contents = std::fs::read_to_string(entry_path.clone()).unwrap();
                    // convert bytecode_contents to Bytes type
                    let bytecode_contents = Bytes::from_str(&bytecode_contents).unwrap();
                    // run bytecode through bytecode analyzer
                    let (events_matches, selector_matches) =
                        bytecode_analyzer::retreive_matches_for_markers(&bytecode_contents);

                    if events_matches.is_some() || selector_matches.is_some() {
                        // add matches to abs_match_output_path
                        let events_to_add: Vec<String> = if events_matches.is_some() {
                            let event_matches = events_matches.unwrap();
                            let event_matches = event_matches.iter();
                            let event_matches = event_matches
                                .map(|index| {
                                    // match index to events position, and then get event.name
                                    events.events[index].name.as_str().to_string()
                                })
                                .collect();
                            event_matches
                        } else {
                            vec![]
                        };

                        let selectors_to_add: Vec<String> = if selector_matches.is_some() {
                            let selector_matches = selector_matches.unwrap();
                            let selector_matches = selector_matches.iter();
                            let selector_matches = selector_matches
                                .map(|index| {
                                    // match index to events position, and then get event.name
                                    selectors.selectors[index].name.as_str().to_string()
                                })
                                .collect();
                            selector_matches
                        } else {
                            vec![]
                        };

                        let contracts = bytecode_analyzer::get_most_similar_contracts(
                            &similar_contracts,
                            &events_to_add,
                            &selectors_to_add,
                        );
                        let most_similar_contracts = MostSimilarContracts {
                            address,
                            most_similar_contracts: contracts,
                        };
                        let contents = serde_json::to_string(&most_similar_contracts).unwrap();
                        return Ok(reply::json(&most_similar_contracts));
                    }

                } else {
                    continue;
                }

                
            }
            Ok(reply::json(&"No matches found"))
        })
        .with(cors)
        ;

    // let contracts = serde_json::from_str::<Vec<ProtocolEventsFns>>(include_str!(
    //     "../../inputs/protocol_events_fns.json"
    // ))
    // .unwrap();

    // let hello = warp::path!("get_similar_contract_for_address" / Address).map(move |address| {
    //     let bytecode = include_str!("../../inputs/{}", address);
    //     let (events_to_add, selectors_to_add) =
    //         bytecode_analyzer::get_events_and_selectors(bytecode);
    //     let contracts = get_most_similar_contracts(&contracts, events_to_add, selectors_to_add);
    // });
    

    warp::serve(get_similar_contracts.or(get_contract_bytecode))
        .run(([0, 0, 0, 0], HTTP_PORT))
        .await;
    Ok(())
}
