use ckb_jsonrpc_types::{BlockNumber, CellOutput, JsonBytes, OutPoint, Script, Uint32};
use serde::{Deserialize, Serialize};

macro_rules! jsonrpc {
    (
        $(#[$struct_attr:meta])*
        pub struct $struct_name:ident {$(
            $(#[$attr:meta])*
            pub fn $method:ident(&mut $selff:ident $(, $arg_name:ident: $arg_ty:ty)*)
                -> $return_ty:ty;
        )*}
    ) => (
        $(#[$struct_attr])*
        pub struct $struct_name {
            pub client: reqwest::Client,
            pub url: reqwest::Url,
            pub id: u64,
        }

        impl $struct_name {
            pub fn new(uri: &str) -> Self {
                let url = reqwest::Url::parse(uri).expect("ckb uri, e.g. \"http://127.0.0.1:8114\"");
                $struct_name { url, id: 0, client: reqwest::Client::new(), }
            }

            $(
                $(#[$attr])*
                pub fn $method(&mut $selff $(, $arg_name: $arg_ty)*) -> Result<$return_ty, failure::Error> {
                    let method = String::from(stringify!($method));
                    let params = serialize_parameters!($($arg_name,)*);
                    $selff.id += 1;

                    let mut req_json = serde_json::Map::new();
                    req_json.insert("id".to_owned(), serde_json::json!($selff.id));
                    req_json.insert("jsonrpc".to_owned(), serde_json::json!("2.0"));
                    req_json.insert("method".to_owned(), serde_json::json!(method));
                    req_json.insert("params".to_owned(), params);

                    let mut resp = $selff.client.post($selff.url.clone()).json(&req_json).send()?;
                    let output = resp.json::<ckb_jsonrpc_types::response::Output>()?;
                    match output {
                        ckb_jsonrpc_types::response::Output::Success(success) => {
                            serde_json::from_value(success.result).map_err(Into::into)
                        },
                        ckb_jsonrpc_types::response::Output::Failure(failure) => {
                            Err(failure.error.into())
                        }
                    }
                }
            )*
        }
    )
}

macro_rules! serialize_parameters {
    () => ( serde_json::Value::Null );
    ($($arg_name:ident,)+) => ( serde_json::to_value(($($arg_name,)+))?)
}

jsonrpc!(pub struct RawHttpRpcClient {

pub fn get_cells(
    &mut self,
    search_key: SearchKey,
    order: Order,
    limit: Uint32,
    after: Option<JsonBytes>
) -> Pagination<Cell>;
});

pub struct IndexerRpcClient {
    _url: String,
    client: RawHttpRpcClient,
}

impl IndexerRpcClient {
    pub fn new(_url: String) -> IndexerRpcClient {
        let client = RawHttpRpcClient::new(_url.as_str());
        IndexerRpcClient { _url, client }
    }
}

impl IndexerRpcClient {
    pub fn get_cells(
        &mut self,
        search_key: SearchKey,
        order: Order,
        limit: Uint32,
        after: Option<JsonBytes>,
    ) -> Result<Pagination<Cell>, String> {
        self.client
            .get_cells(search_key, order, limit, after)
            .map_err(|err| err.to_string())
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct SearchKey {
    pub(crate) script: Script,
    pub(crate) script_type: ScriptType,
    pub(crate) args_len: Option<Uint32>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ScriptType {
    Lock,
    Type,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    Desc,
    Asc,
}

// #[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
// pub struct Tip {
//     pub block_hash: H256,
//     pub block_number: BlockNumber,
// }

// #[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
// pub struct CellsCapacity {
//     pub capacity: Capacity,
//     pub block_hash: H256,
//     pub block_number: BlockNumber,
// }

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct Cell {
    pub output: CellOutput,
    pub output_data: JsonBytes,
    pub out_point: OutPoint,
    pub block_number: BlockNumber,
    pub tx_index: Uint32,
}

// #[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
// pub struct Tx {
//     pub tx_hash: H256,
//     pub block_number: BlockNumber,
//     pub tx_index: Uint32,
//     pub io_index: Uint32,
//     pub io_type: IOType,
// }

// #[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
// #[serde(rename_all = "snake_case")]
// pub enum IOType {
//     Input,
//     Output,
// }

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct Pagination<T> {
    pub objects: Vec<T>,
    pub last_cursor: JsonBytes,
}

pub fn get_live_cell_by_typescript(
    indexer_client: &mut IndexerRpcClient,
    typescript: ckb_types::packed::Script,
) -> Result<Option<Cell>, String> {
    let search_key = SearchKey {
        script: typescript.into(),
        script_type: ScriptType::Type,
        args_len: None,
    };
    let cells = get_live_cells(indexer_client, search_key, |_, _| (true, true))?;
    let len = cells.len();
    if len > 1 {
        return Err("expected zero or one cell".to_string());
    }
    if len == 1 {
        Ok(Some(cells[0].clone()))
    } else {
        Ok(None)
    }
}

pub fn get_live_cells<F: FnMut(usize, &Cell) -> (bool, bool)>(
    indexer_client: &mut IndexerRpcClient,
    search_key: SearchKey,
    mut terminator: F,
) -> Result<Vec<Cell>, String> {
    let limit = Uint32::from(100u32);
    let mut infos = Vec::new();
    let mut cursor = None;
    loop {
        let live_cells: Pagination<Cell> =
            indexer_client.get_cells(search_key.clone(), Order::Asc, limit, cursor)?;
        if live_cells.objects.is_empty() {
            break;
        }
        cursor = Some(live_cells.last_cursor);
        for (index, cell) in live_cells.objects.into_iter().enumerate() {
            let (stop, push_info) = terminator(index, &cell);
            if push_info {
                infos.push(cell);
            }
            if stop {
                return Ok(infos);
            }
        }
    }

    Ok(infos)
}
