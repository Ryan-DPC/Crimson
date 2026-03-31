use lcu_commands::lcu;

#[tokio::main]
async fn main() {
    println!("Fetching Ranked Data...");
    if let Ok(info) = lcu::get_lcu_info() {
        // Try puuid
        if let Ok(sum) = lcu::lcu_request("GET".into(), "/lol-summoner/v1/current-summoner".into(), None).await {
            let j: serde_json::Value = serde_json::from_str(&sum).unwrap();
            let puuid = j["puuid"].as_str().unwrap();
            let sum_id = j["summonerId"].as_u64().unwrap();
            println!("PUUID: {}, SummonerID: {}", puuid, sum_id);
            
            println!("\nTesting /lol-ranked/v1/current-ranked-stats");
            let r1 = lcu::lcu_request("GET".into(), "/lol-ranked/v1/current-ranked-stats".into(), None).await;
            println!("Output: {:?}", r1);
            
            println!("\nTesting /lol-ranked/v1/ranked-stats/{{puuid}}");
            let r2 = lcu::lcu_request("GET".into(), format!("/lol-ranked/v1/ranked-stats/{}", puuid), None).await;
            println!("Output: {:?}", r2);
        }
    } else {
        println!("LCU blocked");
    }
}
