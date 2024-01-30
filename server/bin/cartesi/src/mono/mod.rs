mod balance_manager;
mod http_dispatcher;
mod ingame_wallet_manager;
mod service;
use atb::logging::init_logger;

use service::rollup;

pub fn run() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime needed to continue. qed");

    rt.block_on(async {
        init_logger("warn, cartesi=info, cartesi=debug , domain=debug", true);
    });

    let mt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime needed to continue. qed");

    mt.block_on(async { rollup().await });
}
