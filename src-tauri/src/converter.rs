use regex::Regex;
use serde::{Deserialize, Serialize};
use log::{debug, info, warn, error};

fn default_game_number() -> String {
    "unknown".to_string()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OhhFile {
    pub ohh: OhhHand,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OhhHand {
    pub spec_version: Option<String>,
    #[serde(default = "default_game_number")]
    pub game_number: String,
    pub game_type: Option<String>,
    pub bet_limit: Option<BetLimit>,
    pub small_blind_amount: f64,
    pub big_blind_amount: f64,
    pub currency: Option<String>,
    pub start_date_utc: String,
    pub table_name: String,
    pub table_size: u8,
    pub table_handle: Option<String>,
    pub dealer_seat: u8,
    #[serde(default, deserialize_with = "deserialize_optional_player_id")]
    pub hero_player_id: Option<String>,
    pub site_name: Option<String>,
    pub network_name: Option<String>,
    pub players: Vec<Player>,
    pub rounds: Vec<Round>,
    pub pots: Vec<Pot>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BetLimit {
    pub bet_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Player {
    #[serde(deserialize_with = "deserialize_player_id")]
    pub id: String,
    pub seat: u8,
    pub name: String,
    pub display: Option<String>,
    pub starting_stack: f64,
    pub player_bounty: Option<f64>,
}

fn deserialize_player_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct PlayerIdVisitor;

    impl<'de> Visitor<'de> for PlayerIdVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or integer player ID")
        }

        fn visit_i64<E>(self, value: i64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_u64<E>(self, value: u64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_str<E>(self, value: &str) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_string<E>(self, value: String) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value)
        }
    }

    deserializer.deserialize_any(PlayerIdVisitor)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Round {
    pub id: u8,
    pub street: String,
    #[serde(default)]
    pub cards: Vec<String>,
    pub actions: Vec<Action>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Action {
    pub action_number: u32,
    #[serde(default, deserialize_with = "deserialize_optional_player_id")]
    pub player_id: Option<String>,
    pub action: String,
    pub amount: Option<f64>,
    pub is_allin: Option<bool>,
    pub cards: Option<Vec<String>>,
}

fn deserialize_optional_player_id<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct OptionalPlayerIdVisitor;

    impl<'de> Visitor<'de> for OptionalPlayerIdVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an optional string or integer player ID")
        }

        fn visit_none<E>(self) -> Result<Option<String>, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Option<String>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserialize_player_id(deserializer).map(Some)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Option<String>, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_string()))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Option<String>, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_string()))
        }

        fn visit_str<E>(self, value: &str) -> Result<Option<String>, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_string()))
        }
    }

    deserializer.deserialize_any(OptionalPlayerIdVisitor)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Pot {
    pub number: u8,
    pub amount: f64,
    pub rake: f64,
    pub jackpot: Option<f64>,
    pub player_wins: Vec<PlayerWin>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlayerWin {
    #[serde(deserialize_with = "deserialize_player_id")]
    pub player_id: String,
    pub win_amount: f64,
    pub contributed_rake: Option<f64>,
}

pub fn parse_ohh_chunks(text: &str) -> Result<Vec<OhhHand>, String> {
    debug!("parse_ohh_chunks called with {} bytes", text.len());
    let mut hands = Vec::new();

    for (idx, chunk) in text.split("\n\n").enumerate() {
        let preview = &chunk.chars().take(200).collect::<String>();

        match serde_json::from_str::<OhhFile>(chunk) {
            Ok(data) => {
                debug!("parsed OhhFile chunk {}", idx);
                hands.extend(data.hands);
            }
            Err(e1) => match serde_json::from_str::<OhhHand>(chunk) {
                Ok(hand) => {
                    debug!("parsed OhhHand chunk {}", idx);
                    hands.push(hand);
                }
                Err(e2) => {
                    warn!(
                        "chunk {}: failed to parse (OhhFile: {}, OhhHand: {}). preview: {}",
                        idx, e1, e2, preview
                    );
                }
            },
        }
    }

    if hands.is_empty() {
        return Err("no valid hands could be parsed. please check your file format.".to_string());
    }

    Ok(hands)
}

pub fn fmt_money(x: f64) -> String {
    let abs_val = x.abs();
    let dollars = abs_val as i64;
    let cents = ((abs_val - dollars as f64) * 100.0).round() as i64;

    let dollars_str = if dollars >= 1000 {
        let mut result = String::new();
        let s = dollars.to_string();
        let len = s.len();
        for (i, c) in s.chars().enumerate() {
            if i > 0 && (len - i) % 3 == 0 {
                result.push(',');
            }
            result.push(c);
        }
        result
    } else {
        dollars.to_string()
    };

    let formatted = format!("${}.{:02}", dollars_str, cents);
    if x < 0.0 {
        format!("-{}", formatted)
    } else {
        formatted
    }
}

pub fn card(c: &str) -> String {
    if c.len() >= 2 {
        format!("{}{}", c[0..1].to_uppercase(), c[1..2].to_lowercase())
    } else {
        c.to_string()
    }
}

pub fn cards(board: &[String]) -> String {
    board.iter().map(|c| card(c)).collect::<Vec<_>>().join(" ")
}

pub fn build_header(h: &OhhHand) -> String {
    let game_num = &h.game_number;
    let sb = fmt_money(h.small_blind_amount);
    let bb = fmt_money(h.big_blind_amount);
    let cur = h.currency.as_deref().unwrap_or("USD");

    let ts = h
        .start_date_utc
        .split('.')
        .next()
        .unwrap_or(&h.start_date_utc)
        .replace('T', " ");

    format!(
        "PokerStars Hand #{}: Hold'em No Limit ({}/{} {}) - {} UTC",
        game_num, sb, bb, cur, ts
    )
}

pub fn build_table(h: &OhhHand) -> String {
    format!(
        "Table '{}' {}-max Seat #{} is the button",
        h.table_name, h.table_size, h.dealer_seat
    )
}

pub fn build_seats(h: &OhhHand) -> String {
    let mut players = h.players.clone();
    players.sort_by_key(|p| p.seat);

    players
        .iter()
        .map(|p| {
            format!(
                "Seat {}: {} ({} in chips)",
                p.seat,
                p.name,
                fmt_money(p.starting_stack)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn name_by_id(h: &OhhHand, pid: &str) -> String {
    h.players
        .iter()
        .find(|p| p.id == pid)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Unknown".to_string())
}

pub fn seat_by_id(h: &OhhHand, pid: &str) -> u8 {
    h.players
        .iter()
        .find(|p| p.id == pid)
        .map(|p| p.seat)
        .unwrap_or(0)
}

pub fn street_header(street: &str, brd: &[String], all_board: &[String]) -> String {
    match street {
        "Preflop" => "*** HOLE CARDS ***".to_string(),
        "Flop" => format!("*** FLOP *** [{}]", cards(brd)),
        "Turn" => {
            if all_board.len() >= 4 {
                let flop: Vec<String> = all_board[..3].to_vec();
                let turn = &all_board[3];
                format!("*** TURN *** [{}] [{}]", cards(&flop), card(turn))
            } else {
                format!("*** TURN *** [{}]", cards(brd))
            }
        }
        "River" => {
            if all_board.len() >= 5 {
                let flop_turn: Vec<String> = all_board[..4].to_vec();
                let river = &all_board[4];
                format!("*** RIVER *** [{}] [{}]", cards(&flop_turn), card(river))
            } else {
                format!("*** RIVER *** [{}]", cards(brd))
            }
        }
        "Showdown" => "*** SHOW DOWN ***".to_string(),
        _ => String::new(),
    }
}

pub fn act_line_with_context(
    h: &OhhHand,
    a: &Action,
    tracker: &std::collections::HashMap<u32, (f64, f64, bool)>,
) -> Option<String> {
    let player_id = a.player_id.as_ref()?;
    let n = name_by_id(h, player_id);
    let act = &a.action;
    let amt = a.amount.unwrap_or(0.0);
    let allin = if a.is_allin.unwrap_or(false) {
        " and is all-in"
    } else {
        ""
    };

    match act.as_str() {
        "Post SB" => Some(format!("{}: posts small blind {}", n, fmt_money(amt))),
        "Post BB" => Some(format!("{}: posts big blind {}", n, fmt_money(amt))),
        "Post Ante" => Some(format!("{}: posts the ante {}", n, fmt_money(amt))),
        "Dealt Cards" => {
            let should_show = match &h.hero_player_id {
                Some(hero_id) => hero_id == player_id,
                None => true,
            };

            if should_show {
                if let Some(card_list) = &a.cards {
                    if card_list.len() >= 2 {
                        return Some(format!(
                            "Dealt to {} [{} {}]",
                            n,
                            card(&card_list[0]),
                            card(&card_list[1])
                        ));
                    }
                }
            }
            None
        }
        "Fold" => Some(format!("{}: folds", n)),
        "Check" => Some(format!("{}: checks", n)),
        "Call" => Some(format!("{}: calls {}{}", n, fmt_money(amt), allin)),
        "Bet" => {
            if let Some((prev_bet, _total, has_bet)) = tracker.get(&a.action_number) {
                if *has_bet && *prev_bet > 0.0 {
                    let raise_amount = amt - prev_bet;
                    return Some(format!(
                        "{}: raises {} to {}{}",
                        n,
                        fmt_money(raise_amount),
                        fmt_money(amt),
                        allin
                    ));
                }
            }
            Some(format!("{}: bets {}{}", n, fmt_money(amt), allin))
        }
        "Raise" => {
            if let Some((prev_bet, _total, _has_bet)) = tracker.get(&a.action_number) {
                if *prev_bet > 0.0 {
                    let raise_amount = amt - prev_bet;
                    return Some(format!(
                        "{}: raises {} to {}{}",
                        n,
                        fmt_money(raise_amount),
                        fmt_money(amt),
                        allin
                    ));
                }
            }
            Some(format!("{}: bets {}{}", n, fmt_money(amt), allin))
        }
        "Shows Cards" => {
            if let Some(card_list) = &a.cards {
                if card_list.len() >= 2 {
                    return Some(format!(
                        "{}: shows [{} {}]",
                        n,
                        card(&card_list[0]),
                        card(&card_list[1])
                    ));
                }
            }
            Some(format!("{}: shows", n))
        }
        "Muck" => Some(format!("{}: mucks hand", n)),
        _ => None,
    }
}

pub fn summarize(h: &OhhHand) -> String {
    if h.pots.is_empty() {
        return "*** SUMMARY ***\nTotal pot $0.00 | Rake $0.00".to_string();
    }

    let pot = &h.pots[0];
    let rake = pot.rake;
    let total = pot.amount;

    let mut board = Vec::new();
    for r in &h.rounds {
        if matches!(r.street.as_str(), "Flop" | "Turn" | "River") {
            board.extend(r.cards.clone());
        }
    }

    let mut lines = vec!["*** SUMMARY ***".to_string()];
    lines.push(format!(
        "Total pot {} | Rake {}",
        fmt_money(total),
        fmt_money(rake)
    ));

    if !board.is_empty() {
        lines.push(format!("Board [{}]", cards(&board)));
    }

    for w in &pot.player_wins {
        let seat = seat_by_id(h, &w.player_id);
        let name = name_by_id(h, &w.player_id);
        lines.push(format!(
            "Seat {}: {} collected ({})",
            seat,
            name,
            fmt_money(w.win_amount)
        ));
    }

    lines.join("\n")
}

pub fn ohh_to_pokerstars_text(h: &OhhHand) -> String {
    let mut lines = Vec::new();

    lines.push(build_header(h));
    lines.push(build_table(h));
    lines.push(build_seats(h));

    let mut all_board = Vec::new();
    let mut street_pot_tracker = std::collections::HashMap::new();

    for round in &h.rounds {
        let street = &round.street;
        let brd = &round.cards;

        all_board.extend(brd.clone());

        let mut last_bet_amount: f64 = 0.0;
        let mut has_bet_this_street = false;
        let is_preflop = street == "Preflop";

        let mut blind_lines = Vec::new();
        let mut dealt_lines = Vec::new();
        let mut other_lines = Vec::new();

        for action in &round.actions {
            if let Some(amt) = action.amount {
                match action.action.as_str() {
                    "Post SB" | "Post BB" | "Post Ante" => {
                        last_bet_amount = amt;
                    }
                    "Bet" | "Raise" => {
                        let prev_bet = last_bet_amount;
                        last_bet_amount = amt;

                        street_pot_tracker
                            .insert(action.action_number, (prev_bet, amt, has_bet_this_street));
                        has_bet_this_street = true;
                    }
                    _ => {}
                }
            }

            if let Some(line) = act_line_with_context(h, action, &street_pot_tracker) {
                match action.action.as_str() {
                    "Post SB" | "Post BB" | "Post Ante" => blind_lines.push(line),
                    "Dealt Cards" => dealt_lines.push(line),
                    _ => other_lines.push(line),
                }
            }
        }

        if is_preflop {
            for line in blind_lines {
                lines.push(line);
            }

            let header = street_header(street, brd, &all_board);
            if !header.is_empty() {
                lines.push(header);
            }

            for line in dealt_lines {
                lines.push(line);
            }

            for line in other_lines {
                lines.push(line);
            }
        } else {
            let header = street_header(street, brd, &all_board);
            if !header.is_empty() {
                lines.push(header);
            }

            for line in blind_lines {
                lines.push(line);
            }
            for line in dealt_lines {
                lines.push(line);
            }
            for line in other_lines {
                lines.push(line);
            }
        }
    }

    lines.push(summarize(h));

    lines.join("\n")
}

pub fn convert_ohh_file(content: &str) -> Result<String, String> {
    debug!("convert_ohh_file called with {} bytes", content.len());

    let hands = parse_ohh_chunks(content)
        .map_err(|e| format!("failed to parse ohh content: {}", e))?;

    debug!("converting {} hands to pokerstars format", hands.len());
    let converted_hands: Vec<String> = hands.iter().map(ohh_to_pokerstars_text).collect();
    let result = converted_hands.join("\n\n\n\n");

    if result.trim().is_empty() {
        return Err("conversion produced no output. check file formatting.".to_string());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt_money() {
        assert_eq!(fmt_money(10.5), "$10.50");
        assert_eq!(fmt_money(1000.0), "$1,000.00");
        assert_eq!(fmt_money(0.05), "$0.05");
        assert_eq!(fmt_money(-5.0), "-$5.00");
    }

    #[test]
    fn test_card() {
        assert_eq!(card("Ah"), "Ah");
        assert_eq!(card("td"), "Td");
        assert_eq!(card("2c"), "2c");
    }

    #[test]
    fn test_cards() {
        let board = vec!["Ah".to_string(), "Kd".to_string(), "Qc".to_string()];
        assert_eq!(cards(&board), "Ah Kd Qc");
    }

    #[test]
    fn test_hand_conversion_with_dealt_cards() {
        let input = r#"{"ohh":{"spec_version":"1.4.3","site_name":"iPoker","game_number":"test123","start_date_utc":"2023-12-05T02:50:49.886Z","table_name":"TestTable","table_size":6,"dealer_seat":3,"small_blind_amount":0.05,"big_blind_amount":0.1,"currency":"PPC","players":[{"id":1,"seat":1,"name":"Player1","starting_stack":10.0},{"id":2,"seat":2,"name":"Player2","starting_stack":10.0}],"rounds":[{"id":0,"street":"Preflop","cards":[],"actions":[{"action_number":0,"player_id":1,"action":"Dealt Cards","cards":["As","Kd"]},{"action_number":1,"player_id":1,"action":"Post SB","amount":0.05},{"action_number":2,"player_id":2,"action":"Post BB","amount":0.1},{"action_number":3,"player_id":1,"action":"Raise","amount":0.3},{"action_number":4,"player_id":2,"action":"Fold"}]}],"pots":[{"number":0,"amount":0.2,"rake":0,"player_wins":[{"player_id":1,"win_amount":0.2}]}]}}"#;

        let result = convert_ohh_file(input);
        assert!(result.is_ok(), "Conversion should succeed");

        let output = result.unwrap();
        println!("\n=== TEST OUTPUT ===\n{}\n=== END ===\n", output);

        // Verify key components
        assert!(
            output.contains("PokerStars Hand #test123"),
            "Should have header"
        );
        assert!(
            output.contains("Player1: posts small blind $0.05"),
            "Should have SB"
        );
        assert!(
            output.contains("Player2: posts big blind $0.10"),
            "Should have BB"
        );
        assert!(
            output.contains("*** HOLE CARDS ***"),
            "Should have hole cards header"
        );
        assert!(
            output.contains("Dealt to Player1 [As Kd]"),
            "Should show dealt cards"
        );
        assert!(
            output.contains("Player1: raises"),
            "Should have raise action"
        );
        assert!(output.contains("Player2: folds"), "Should have fold");
        assert!(output.contains("*** SUMMARY ***"), "Should have summary");
    }

    #[test]
    fn test_raise_formatting() {
        let input = r#"{"ohh":{"spec_version":"1.4.3","site_name":"iPoker","game_number":"raise_test","start_date_utc":"2023-12-05T02:50:49.886Z","table_name":"TestTable","table_size":6,"dealer_seat":3,"small_blind_amount":0.05,"big_blind_amount":0.1,"currency":"USD","players":[{"id":1,"seat":1,"name":"Player1","starting_stack":10.0},{"id":2,"seat":2,"name":"Player2","starting_stack":10.0}],"rounds":[{"id":0,"street":"Preflop","cards":[],"actions":[{"action_number":0,"player_id":1,"action":"Post SB","amount":0.05},{"action_number":1,"player_id":2,"action":"Post BB","amount":0.1},{"action_number":2,"player_id":1,"action":"Raise","amount":0.3},{"action_number":3,"player_id":2,"action":"Call","amount":0.3}]},{"id":1,"street":"Flop","cards":["Ah","Kd","Qc"],"actions":[{"action_number":0,"player_id":1,"action":"Check"},{"action_number":1,"player_id":2,"action":"Bet","amount":0.5},{"action_number":2,"player_id":1,"action":"Raise","amount":1.5}]}],"pots":[{"number":0,"amount":3.6,"rake":0,"player_wins":[{"player_id":1,"win_amount":3.6}]}]}}"#;

        let result = convert_ohh_file(input);
        assert!(result.is_ok(), "Conversion should succeed");

        let output = result.unwrap();
        println!("\n=== RAISE TEST OUTPUT ===\n{}\n=== END ===\n", output);

        // Check that raises are formatted correctly
        assert!(
            output.contains("raises") && output.contains("to"),
            "Raises should use 'raises X to Y' format"
        );
    }

    #[test]
    fn test_real_sample_hand() {
        // This is an actual hand from the sample file
        let input = r#"{"ohh":{"spec_version":"1.4.3","site_name":"iPoker","network_name":"iPoker Network","internal_version":"1.0.0","tournament":false,"game_number":"lv0irhede81k","start_date_utc":"2023-12-05T02:50:49.886Z","table_name":"pglCX2WsUJbPBjsNSE1siiDJy","table_handle":"pglCX2WsUJbPBjsNSE1siiDJy","table_skin":"","game_type":"Holdem","bet_limit":{"bet_type":"NL","bet_cap":0},"table_size":10,"currency":"PPC","dealer_seat":8,"small_blind_amount":0.05,"big_blind_amount":0.1,"ante_amount":0,"flags":["Observed"],"players":[{"id":1,"seat":1,"name":"Agapito","display":"Agapito","starting_stack":19.9,"player_bounty":0},{"id":4,"seat":4,"name":"DubNation","display":"DubNation","starting_stack":9.8,"player_bounty":0},{"id":5,"seat":5,"name":"CFFl2rCOze","display":"bella","starting_stack":11.2,"player_bounty":0},{"id":6,"seat":6,"name":"-c6EEVvXCE","display":"bdawg","starting_stack":10,"player_bounty":0},{"id":7,"seat":7,"name":"E9V-2MDLwt","display":"Redorange","starting_stack":10.55,"player_bounty":0},{"id":8,"seat":8,"name":"JzhSREGpIj","display":"Drank","starting_stack":8.55,"player_bounty":0}],"rounds":[{"id":0,"street":"Preflop","cards":[],"actions":[{"action_number":0,"player_id":4,"action":"Dealt Cards","cards":["Ks","2c"],"is_allin":false},{"action_number":1,"player_id":6,"action":"Dealt Cards","cards":["8s","Ac"],"is_allin":false},{"action_number":2,"player_id":1,"action":"Post SB","amount":0.05,"is_allin":false},{"action_number":3,"player_id":4,"action":"Post BB","amount":0.1,"is_allin":false},{"action_number":4,"player_id":5,"action":"Fold","amount":0,"is_allin":false},{"action_number":5,"player_id":6,"action":"Raise","amount":0.22,"is_allin":false},{"action_number":6,"player_id":7,"action":"Fold","amount":0,"is_allin":false},{"action_number":7,"player_id":8,"action":"Fold","amount":0,"is_allin":false},{"action_number":8,"player_id":1,"action":"Fold","amount":0,"is_allin":false},{"action_number":9,"player_id":4,"action":"Call","amount":0.12,"is_allin":false}]},{"id":1,"cards":["4d","3c","Kd"],"street":"Flop","actions":[{"action_number":0,"player_id":4,"action":"Check","amount":0,"is_allin":false},{"action_number":1,"player_id":6,"action":"Raise","amount":0.24,"is_allin":false},{"action_number":2,"player_id":4,"action":"Call","amount":0.24,"is_allin":false}]},{"id":2,"cards":["Tc"],"street":"Turn","actions":[{"action_number":0,"player_id":4,"action":"Check","amount":0,"is_allin":false},{"action_number":1,"player_id":6,"action":"Check","amount":0,"is_allin":false}]},{"id":3,"cards":["Js"],"street":"River","actions":[{"action_number":0,"player_id":4,"action":"Raise","amount":0.48,"is_allin":false},{"action_number":1,"player_id":6,"action":"Raise","amount":1.5,"is_allin":false},{"action_number":2,"player_id":4,"action":"Call","amount":1.02,"is_allin":false},{"action_number":3,"player_id":4,"action":"Shows Cards","cards":["Ks","2c"],"is_allin":false},{"action_number":4,"player_id":6,"action":"Shows Cards","cards":["8s","Ac"],"is_allin":false}]}],"pots":[{"number":0,"amount":3.97,"rake":0,"jackpot":null,"player_wins":[{"player_id":4,"win_amount":3.97,"contributed_rake":0}]}]}}"#;

        let result = convert_ohh_file(input);
        assert!(result.is_ok(), "Real hand conversion should succeed");

        let output = result.unwrap();
        println!("\n=== REAL HAND OUTPUT ===\n{}\n=== END ===\n", output);

        // Verify the output has all expected components
        assert!(
            output.contains("Dealt to DubNation"),
            "Should show DubNation's cards"
        );
        assert!(
            output.contains("Dealt to -c6EEVvXCE"),
            "Should show bdawg's cards"
        );
        assert!(
            output.contains("*** FLOP *** [4d 3c Kd]"),
            "Should have flop"
        );
        assert!(output.contains("*** TURN ***"), "Should have turn");
        assert!(output.contains("*** RIVER ***"), "Should have river");
        assert!(
            output.contains("DubNation: shows [Ks 2c]"),
            "Should show showdown cards"
        );
        assert!(
            output.contains("-c6EEVvXCE: shows [8s Ac]"),
            "Should show showdown cards"
        );
    }

    #[test]
    #[ignore]
    fn test_full_sample_file() {
        use std::fs;

        println!("\n=== CONVERTING FULL SAMPLE FILE ===\n");

        let content = fs::read_to_string("../hands-pglCX2WsUJbPBjsNSE1siiDJy.ohh.txt")
            .expect("Failed to read sample file. Make sure to run from src-tauri directory.");

        println!("Loaded {} bytes from sample file", content.len());
        println!("Converting hands...\n");

        let result = convert_ohh_file(&content);

        assert!(result.is_ok(), "Conversion failed: {:?}", result.err());

        let output = result.unwrap();
        let hand_count = output.matches("PokerStars Hand #").count();

        println!("Successfully converted {} hands", hand_count);
        println!(
            "Output size: {} bytes ({:.1} KB)",
            output.len(),
            output.len() as f64 / 1024.0
        );

        fs::write("../converted_full_sample.txt", &output).expect("Failed to write output file");

        println!("Wrote converted hands to converted_full_sample.txt\n");

        let preview: Vec<&str> = output.lines().take(50).collect();
        println!("=== PREVIEW (first 50 lines) ===");
        for line in preview {
            println!("{}", line);
        }
        println!("=== END PREVIEW ===\n");

        println!("Full output saved to: converted_full_sample.txt");
    }
}
