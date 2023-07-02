use binance::api::Binance;
use binance::general::General;
use binance::market::Market;
use binance::websockets::{WebSockets, WebsocketEvent};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use strategy_backtester::backtest::*;
use strategy_backtester::patterns::*;
use strategy_backtester::strategies::*;
use strategy_backtester::strategies_creator::*;
use strategy_backtester::tools::retreive_test_data;
use strategy_backtester::*;

const START_MONEY: f64 = 100.;
const DATA_FOLDER: &str = "data/";

fn main() {
    /*//let market: Market = Binance::new(None, None);
    //let general: FuturesGeneral = Binance::new(None, None);
    let keep_running = AtomicBool::new(true);*/
 // Used to control the event loop

    let market: Market = Binance::new(None, None);
    let general: General = Binance::new(None, None);
    let endpoints = ["btcusdt@trade".to_string()];
    let keep_running = AtomicBool::new(true);
    let (tx_price, rx_price) = channel::<f64>();
    let (tx_price_2, rx_price_2) = channel::<f64>();
    let (tx_trades, rx_trades) = channel::<Vec<Trade>>();
    let (tx_opened_trades, rx_opened_trades) = channel::<Trade>();
    let number_of_klines: Arc<Mutex<u16>> = Arc::new(Mutex::new(50));
    let number_of_klines_clone = number_of_klines.clone();

    thread::spawn(move || {
        let mut last_data_dl: u64 = 0;
        loop {
            let mut server_time = 0;
            let result = general.get_server_time();
            match result {
                Ok(answer) => {
                    //println!("Server Time: {}", answer.server_time);
                    server_time = answer.server_time;
                }
                Err(e) => println!("Error: {}", e),
            }

            if server_time > last_data_dl + (1_000 * 60) {
                last_data_dl = server_time;
                let tmp_num = *number_of_klines.lock().unwrap();
                *number_of_klines.lock().unwrap() = tmp_num + 1;

                let symbol = "BTCUSDT".to_string();
                let interval = "1m".to_string();
                let klines = retreive_test_data(
                    server_time,
                    &market,
                    symbol,
                    interval,
                    DATA_FOLDER.to_string(),
                    1,
                    *number_of_klines.lock().unwrap(),
                );
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
                        min: 1,
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
                println!("avant test");
                let potential_trades: Vec<Trade> = Backtester::new(arc_klines, None, None, true)
                    .add_strategies(&mut strategies)
                    .start_potential_only()
                    .to_vec();
                println!("après test");
                tx_trades.send(potential_trades);
                println!("après send");
            }
        }
    });

    thread::spawn(move || {
        // dès que ca recoit un nouveau trade, ca le met dans la liste des trades open

        // et dès que ca recoit un changement de prix avec rx_price_2, ca check toute la liste voir si il est closed

    });

    let mut web_socket = WebSockets::new(|event: WebsocketEvent| {
        match event {
            WebsocketEvent::Trade(trade_event) => {
                //println!("Symbol: {}, Price: {}", trade_event.symbol, trade_event.price);
                tx_price.send(trade_event.price.parse::<f64>().unwrap());
                tx_price_2.send(trade_event.price.parse::<f64>().unwrap());
            }
            _ => (),
        };
        Ok(())
    });

    thread::spawn(move || {
        let mut trades = rx_trades.recv().unwrap();
        println!("{} premiers trades potentiels", trades.len());
        loop {
            let current_price = rx_price.recv().unwrap();
            if let Ok(received_trades) = rx_trades.try_recv() {
                trades = received_trades;
                println!("{} trades potentiels", trades.len());
            }
            for trade in &trades[..] {
                if trade.tp > trade.sl && trade.entry_price <= current_price {
                    *number_of_klines_clone.lock().unwrap() = 0;
                    println!("Ca prend un trade long. Current price : {} \n trade : {:#?}", current_price, trade);
                    trades.clear();
                    break;
                } else if trade.tp < trade.sl && trade.entry_price >= current_price {
                    *number_of_klines_clone.lock().unwrap() = 0;
                    println!("Ca prend un trade short. Current price : {} \n trade : {:#?}", current_price, trade);
                    trades.clear();
                    break;
                }
            }
        }
    });

    web_socket.connect_multiple_streams(&endpoints).unwrap(); // check error
    if let Err(e) = web_socket.event_loop(&keep_running) {
        println!("Error: {:?}", e);
    }
    web_socket.disconnect().unwrap();
}
