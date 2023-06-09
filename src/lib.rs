use std::io::{BufRead, StdoutLock, Write};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message<P> {
    pub src: String,
    #[serde(rename = "dest")]
    pub dst: String,
    pub body: Body<P>,
}
impl<P> Message<P>{
    pub fn into_reply(self,id:Option<&mut usize>)->Self{
        Self{
            src: self.dst,
            dst: self.src,
            body: Body {
                id: id.map(|id|{
                    let mid = *id;
                    *id+=1;
                    mid
                }),
                in_reply_to: self.body.id,
                payload: self.body.payload
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Body<P> {
    #[serde(rename = "msg_id")]
    pub id: Option<usize>,
    pub in_reply_to: Option<usize>,
    #[serde(flatten)]
    pub payload: P,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Init {
    pub node_id: String,
    pub node_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum InitPayload {
    Init(Init),
    InitOk,
}
// pub trait Payload:Sized{
//     fn extract_init( input:Self)->Option<Init>;
//     fn init_ok() ->Self;
// }

pub trait Node<S, P> {
    fn from_init(state: S, init: Init) -> anyhow::Result<Self> where Self: Sized;
    fn step(&mut self, input: Message<P>, output: &mut StdoutLock) -> anyhow::Result<()>;
}

pub fn main_loop<S, N, P>(init_state: S) -> anyhow::Result<()>
    where

        N: Node<S, P>,
        P: DeserializeOwned
{
    let stdin = std::io::stdin().lock();
    let mut stdin = stdin.lines();
    let init_msg: Message<InitPayload> = serde_json::from_str(
        &stdin
            .next()
            .expect("no init message received")
            .context("failed to read init message from stdin")?
    ).context("init message could not be deserialized")?;
    let InitPayload::Init(init) = init_msg.body.payload else {
        panic!("first message should be init");
    };
    let mut node: N = Node::from_init(init_state, init).context("node initialization failed")?;

    let mut stdout = std::io::stdout().lock();
    let reply = Message {
        src: init_msg.dst,
        dst: init_msg.src,
        body: Body {
            id: Some(0),
            in_reply_to: init_msg.body.id,
            payload: InitPayload::InitOk,
        },
    };

    serde_json::to_writer(&mut stdout, &reply).context("serialize response to init")?;
    stdout.write_all(b"\n").context("write trailing newline")?;
    for line in stdin {
        let line = line.context("Maelstrom input from STDIN could not be read")?;
        let input = serde_json::from_str(&line).context("Maelstrom input from STDIN could not be deserialized")?;
        node.step(input, &mut stdout).context("Node step function failed")?;
    }
    Ok(())
}
