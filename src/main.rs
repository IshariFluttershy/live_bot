use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use binance::api::Binance;
use binance::general::General;
use binance::market::Market;
use binance::websockets::{WebsocketEvent, WebSockets};
use strategy_backtester::backtest::*;
use strategy_backtester::patterns::*;
use strategy_backtester::strategies::*;
use strategy_backtester::strategies_creator::*;
use strategy_backtester::*;
use strategy_backtester::tools::retreive_test_data;

const START_MONEY: f64 = 100.;
const DATA_FOLDER: &str = "data/";

fn main() {
    /*//let market: Market = Binance::new(None, None);
    //let general: FuturesGeneral = Binance::new(None, None);
    let keep_running = AtomicBool::new(true);*/ // Used to control the event loop

    let market: Market = Binance::new(None, None);
    let general: General = Binance::new(None, None);
    let endpoints = ["btcusdt@trade".to_string(), "btcusdt@kline_1m".to_string(), "ethusdt@trade".to_string(), "ethusdt@kline_1m".to_string()];

    let mut server_time = 0;
    let result = general.get_server_time();
    match result {
        Ok(answer) => {
            println!("Server Time: {}", answer.server_time);
            server_time = answer.server_time;
        }
        Err(e) => println!("Error: {}", e),
    }
    let keep_running = AtomicBool::new(true);


    let symbol = "BTCUSDT".to_string();
    let interval = "1m".to_string();
    let klines = retreive_test_data(server_time, &market, symbol, interval, DATA_FOLDER.to_string(), 1, 25);
    let readable_klines = Backtester::to_all_math_kline(klines);
    let arc_klines = Arc::new(readable_klines);
    let mut strategies = create_w_and_m_pattern_strategies(
        START_MONEY,
        ParamMultiplier {
            min: 2.,
            max: 2.,
            step: 1.,
        },
        ParamMultiplier {
            min: 1.,
            max: 1.,
            step: 2.,
        },
        ParamMultiplier {
            min: 3,
            max: 3,
            step: 1,
        },
        ParamMultiplier {
            min: 25,
            max: 25,
            step: 5,
        },
        ParamMultiplier {
            min: 1.,
            max: 1.,
            step: 1.,
        },
        MarketType::Spot,
    );

    let potential_trades = Backtester::new(arc_klines, None, None, true)
    .add_strategies(&mut strategies)
    .start_potential_only();

    let mut web_socket = WebSockets::new(|event: WebsocketEvent| {
        match event {
            WebsocketEvent::Trade(trade_event) => {
                println!("Symbol: {}, Price: {}", trade_event.symbol, trade_event.price);
                // Ici mettre a jour la variable de prix
                // genre current_price = trade_event.price;
            },
            _ => (),
        };
        Ok(())
    });

    web_socket.connect_multiple_streams(&endpoints).unwrap(); // check error
    if let Err(e) = web_socket.event_loop(&keep_running) {
        println!("Error: {:?}", e);
    }
    web_socket.disconnect().unwrap();
}
