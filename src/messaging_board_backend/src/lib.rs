#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};
use ic_stable_structures::storable::Bound;
//use std::error::Error;
//use candid::Error;

/* Defining memory state and Idcell */
type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

//Defining message struct
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Message {
    id: u64, //we have the massage id
    title: String, //we have our message title
    body: String,   //body of the message
    attachment_url: String, 
    created_at: u64,
    updated_at: Option<u64>,
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error{
    NotFound {msg: String},
}

// a trait that must be implemented for a struct that is stored in a stable struct
impl Storable for Message {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes:std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 1024,
        is_fixed_size: false,
    };
}

thread_local!{
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );
       
       static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(
            |m| m.borrow().get(MemoryId::new(0))),0).expect("cannot create counter")
        );

        static STORAGE: RefCell<StableBTreeMap<u64, Message, Memory>> = RefCell::new(
            StableBTreeMap::init(
                MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
            ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct MessagePayload {
    title: String,
    body: String,
    attachment_url: String,
}

//use core::fmt::Error;

#[ic_cdk::query]
fn get_message(id: u64) -> Result<Message, Error>{
    match _get_message(&id){
        Some(message) => Ok(message),
        None => Err(Error::NotFound{
            msg: format!("a message with id={} not found", id),
        })
    }
}

fn _get_message(id:&u64) -> Option<Message>{
    STORAGE.with(|s| s.borrow().get(id))
}

#[ic_cdk::update]
fn add_message(message: MessagePayload) -> Option<Message> {
    let id = ID_COUNTER.with(|counter| {
        let current_value = *counter.borrow().get();
        counter.borrow_mut().set(current_value + 1)
    }).expect("cannot incremet id counter");

    let message =Message{
        id, 
        title: message.title,
        body: message.body,
        attachment_url: message.attachment_url,
        created_at: time(),
        updated_at: None,
    };

     do_insert(&message);  //sending and storing into the cannister storage
     Some(message)
}

fn do_insert(message: &Message) {
    STORAGE.with(|service| service.borrow_mut().insert(message.id, message.clone())); 
}


#[ic_cdk::update]
fn update_message(id: u64, payload: MessagePayload) -> Result<Message, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)){
        Some(mut message) => {
            message.attachment_url = payload.attachment_url;
            message.body = payload.body;
            message.title =payload.title;
            message.updated_at = Some(time());
            do_insert(&message);
            Ok(message)
        }
        None => Err(Error::NotFound{
            msg: format!(
                "could not update a message with id={}. message not found", id
            )
        })
    }
}

#[ic_cdk::update]
fn delete_message(id:u64) -> Result<Message, Error> {
    match STORAGE.with(|service| service.borrow_mut().remove(&id)){
        Some(message) => Ok(message),
        None => Err(Error::NotFound { 
            msg: format!("could not delete a message with id={}. message not found", id) 
        })
    }
}

ic_cdk::export_candid!();
