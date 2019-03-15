macro_rules! generate_module_for {
    ([$( $name:ident, )+]) => {
        $( generate_module_for!($name); )+
    };
    ([$( $name:ident ),+]) => {
        $( generate_module_for!($name); )+
    };
    ($name:ident) => {
        pub mod $name {
            include!(concat!(env!("OUT_DIR"), "/", stringify!($name), ".rs"));
        }
    };
}

generate_module_for!([
    block,
    transaction,
    receipt,
    blockchain,
    chain,
    common,
    consensus,
    executor,
    pool,
    sync
]);
