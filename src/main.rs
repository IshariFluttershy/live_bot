use std::sync::atomic::AtomicBool;
use binance::websockets::{WebsocketEvent, WebSockets};
use strategy_backtester::backtest::*;
use strategy_backtester::patterns::*;
use strategy_backtester::strategies::*;
use strategy_backtester::strategies_creator::*;
use strategy_backtester::*;

const START_MONEY: f64 = 100.;

fn main() {
    //let market: Market = Binance::new(None, None);
    //let general: FuturesGeneral = Binance::new(None, None);
    let keep_running = AtomicBool::new(true); // Used to control the event loop
    /*let endpoints = ["kline_1m"]
        .map(|symbol| format!("btcusdt@{}", symbol.to_lowercase()));*/
    let kline_endpoint = format!("{}", "btcusdt@kline_1m");
    let mut kline_web_socket = WebSockets::new(|event: WebsocketEvent| {
        match event {
            WebsocketEvent::Kline(kline_event) => {
                println!("Symbol: {}, High: {}, Low {}", kline_event.symbol, kline_event.kline.high, kline_event.kline.low);
            }
            _ => (),
        };
        Ok(())
    });   

    kline_web_socket.connect(&kline_endpoint).unwrap(); // check error
    if let Err(e) = kline_web_socket.event_loop(&keep_running) {
        match e {
          err => {
             println!("Error: {:?}", err);
          }
        }
    }
    kline_web_socket.disconnect().unwrap();



    let trades_endpoint = format!("{}", "btcusdt@trades");
    let mut trades_web_socket = WebSockets::new(|event: WebsocketEvent| {
        match event {
            WebsocketEvent::Trade(trade_event) => {
                println!("Symbol: {}, Price: {}", trade_event.symbol, trade_event.price);
            },
            _ => (),
        };
        Ok(())
    });   

    trades_web_socket.connect(&trades_endpoint).unwrap(); // check error
    if let Err(e) = trades_web_socket.event_loop(&keep_running) {
        match e {
          err => {
             println!("Error: {:?}", err);
          }
        }
    }
    trades_web_socket.disconnect().unwrap();



    /*let strategies = create_w_and_m_pattern_strategies(
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
    Backtester::new(arc_klines_clone, tx_clone, i)
                .add_strategies(&mut strategies)
                .start()
                .get_results();*/

}
