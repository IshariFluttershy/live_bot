use binance::api::Binance;
use binance::general::General;
use binance::market::Market;
use binance::model::Kline;
use binance::websockets::{WebSockets, WebsocketEvent};
use std::fs::read;
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
use std::time::{SystemTime, UNIX_EPOCH};

const START_MONEY: f64 = 100.;
const DATA_FOLDER: &str = "data/";

fn main() {
    /*//let market: Market = Binance::new(None, None);
    //let general: FuturesGeneral = Binance::new(None, None);
    let keep_running = AtomicBool::new(true);*/
 // Used to control the event loop

    let market: Market = Binance::new(None, None);
    let general: General = Binance::new(None, None);
    let endpoints = ["btcusdt@trade".to_string(), "btcusdt@kline_1m".to_string()];
    let keep_running = AtomicBool::new(true);
    let (tx_price, rx_price) = channel::<f64>();
    let (tx_price_2, rx_price_2) = channel::<f64>();
    let (tx_trades, rx_trades) = channel::<Vec<Trade>>();
    let (tx_opened_trades, rx_opened_trades) = channel::<Trade>();
    let (tx_kline, rx_kline) = channel::<MathKLine>();
    let number_of_klines: Arc<Mutex<u16>> = Arc::new(Mutex::new(26));
    let number_of_klines_clone = number_of_klines.clone();
    let max_klines_batch = 70;
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    thread::spawn(move || {
        let klines: &mut Vec<MathKLine> = &mut Vec::new();
        loop {
            let new_kline = rx_kline.recv().unwrap();
            
            
            if !klines.is_empty() && new_kline.open_time != klines.last().unwrap().close_time + 1{
                println!("Error, new kline opening time doesn't follow previous kline closing time");
                klines.clear();
            }
            klines.push(new_kline);
            println!("kline received, klines length == {}", klines.len());

            println!("klines retreived : {:#?}", klines);
            let arc_klines: Arc<Vec<MathKLine>> = Arc::new(klines.to_vec());
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
            let potential_trades: Vec<Trade> = Backtester::new(arc_klines, None, None, true)
                .add_strategies(&mut strategies)
                .start_potential_only()
                .to_vec();
            tx_trades.send(potential_trades);
        }
    });

    thread::spawn(move || {
        let mut opened_trades: Vec<Trade> = Vec::new();
        let mut current_price = 0.;
        let mut total_money = START_MONEY;

        loop {
            if let Ok(received_trade) = rx_opened_trades.try_recv() {
                opened_trades.push(received_trade);
                println!("{} trades ouverts", opened_trades.len());
            }
            current_price = rx_price_2.recv().unwrap();

            for i in 0..opened_trades.len() {
                let trade = &opened_trades[i];
                if trade.tp > trade.sl {
                    if current_price <= trade.sl {
                        total_money -= trade.loss;
                        opened_trades.swap_remove(i);
                        println!("Trade closed. LOSE Total money is now : {}", total_money);
                    } else if current_price >= trade.tp {
                        total_money += trade.benefits;
                        opened_trades.swap_remove(i);
                        println!("Trade closed. WIN Total money is now : {}", total_money);
                    }
                } else if trade.tp < trade.sl {
                    if current_price >= trade.sl {
                        total_money -= trade.loss;
                        opened_trades.swap_remove(i);
                        println!("Trade closed. LOSE Total money is now : {}", total_money);
                    } else if current_price <= trade.tp {
                        total_money += trade.benefits;
                        opened_trades.swap_remove(i);
                        println!("Trade closed. WIN Total money is now : {}", total_money);
                    }
                }
            }
        }
    });

    let mut last_kline_close_time = 0;

    let mut web_socket = WebSockets::new(|event: WebsocketEvent| {
        match event {
            WebsocketEvent::Trade(trade_event) => {
                //println!("Symbol: {}, Price: {}", trade_event.symbol, trade_event.price);
                tx_price.send(trade_event.price.parse::<f64>().unwrap());
                tx_price_2.send(trade_event.price.parse::<f64>().unwrap());
            },
            WebsocketEvent::Kline(kline_event) => {
                if last_kline_close_time == 0 || last_kline_close_time + 1 == kline_event.kline.open_time {
                    last_kline_close_time = kline_event.kline.close_time;
                    tx_kline.send(Backtester::kline_to_math_kline(&kline_event.kline));
                }
                    // ICI 
                // prendre le closing time de la dernière kline et le comparer avec l'open time de la kline recue a l'instant.
                // Si l'open time de la kline recue est le close time + 1 alors, ajouter la kline dans la liste et mettre a jour le closing time de la dernière kline avec la nouvelle

                
                println!("Kline event received : {:#?}", kline_event.kline.open_time);
            },
            _ => (),
        };
        Ok(())
    });

    thread::spawn(move || {
        let mut trades: Vec<(Trade, u128)> = Vec::new();
        loop {
            let current_price = rx_price.recv().unwrap();
            let current_time = since_the_epoch.as_millis();
            if let Ok(received_trades) = rx_trades.try_recv() {
                for trade in received_trades {
                    trades.push((trade, current_time));
                }
                println!("{} trades potentiels", trades.len());
            }
            let mut i = 0;
            while i < trades.len() {
                let (trade, opening_time) = &trades[i];
                if trade.tp > trade.sl {
                    if trade.entry_price <= current_price {
                        *number_of_klines_clone.lock().unwrap() = 0;
                        println!("Ca prend un trade long. Current price : {} \n trade : {:#?}", current_price, trade);
                        let mut clone = trade.clone();
                        clone.loss = START_MONEY * 0.01 * 1.;
                        clone.benefits = START_MONEY * 0.01 * 2.;
                        tx_opened_trades.send(clone);
                        trades.clear();
                        break;
                    } else if current_price < trade.sl {
                        trades.swap_remove(i);
                    }
                } else if trade.tp < trade.sl {
                    if trade.entry_price >= current_price {
                        *number_of_klines_clone.lock().unwrap() = 0;
                        println!("Ca prend un trade short. Current price : {} \n trade : {:#?}", current_price, trade);
                        let mut clone = trade.clone();
                        clone.loss = START_MONEY * 0.01 * 1.;
                        clone.benefits = START_MONEY * 0.01 * 2.;
                        tx_opened_trades.send(clone);
                        trades.clear();
                        break;
                    } else if current_price > trade.sl {
                        trades.swap_remove(i);
                    }
                } else if current_time > opening_time + 30 * 60 * 1000 {
                    trades.swap_remove(i);
                }
                i += 1;
            }
        }
    });

    web_socket.connect_multiple_streams(&endpoints).unwrap(); // check error
    if let Err(e) = web_socket.event_loop(&keep_running) {
        println!("Error: {:?}", e);
    }
    web_socket.disconnect().unwrap();
}
