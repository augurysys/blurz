use dbus::arg;
use bluetooth_utils;
// use dbus::blocking::Proxy;
use dbus::{Connection, MessageItem, Message};
use bluetooth_session::BluetoothSession;
use dbus::tree::Factory;
use std::sync::Arc;
use std::error::Error;
use std::time::Duration;


// extern crate dbus;


const AGENT_INTERFACE: &str = "org.bluez.AgentManager1";
const REG_AGENT: &str = "RegisterAgent";
// static SERVICE_NAME: &'static str = "org.bluez";
pub(crate) const MANAGER_PATH: &str = "/org/bluez/agent";

#[derive(Debug)]
pub struct BluetoothAgent<'a> {
    path: String,
    adapter: String,
    conn : Connection,
    session: &'a BluetoothSession,
}
use dbus::{BusType};
use std::sync::{Mutex};

// Define the agent state (registered or not)
struct AgentState {
    registered: bool,
    agent_path: String,
}

impl<'a> BluetoothAgent <'a> {
    fn new(path: String, adapter: String, session: &'a BluetoothSession) -> BluetoothAgent<'a> {
        BluetoothAgent {
            path: path,
            adapter: adapter,
            session: session,
            conn: Connection::get_private(dbus::BusType::System).unwrap(),
        }
    }
    pub fn check_agent_manager_available() -> Result<bool, Box<dyn std::error::Error>> {
        // Connect to the system bus
        let conn = Connection::get_private(dbus::BusType::System)?;
    
        // Define the BlueZ service and the object path
        let bluez_service = "org.bluez";
        let agent_manager_path = "/org/bluez/agent_manager";  // Replace with the correct path
    
        // Introspect the object path to see which interfaces are available
        let msg = Message::new_method_call(bluez_service, agent_manager_path, "org.freedesktop.DBus.Introspectable", "Introspect")?;
    
        // Send the introspection message
        let response: Message = conn.send_with_reply_and_block(msg, 1000)?;
    
        // Read the introspection data, which is XML describing the available interfaces
        let introspection_data: String = response.read1()?;
        println!("Introspection data: {}", introspection_data);
        // Check if the org.bluez.AgentManager1 interface is in the introspection data
        if introspection_data.contains("org.bluez.AgentManager1") {
            println!("The org.bluez.AgentManager1 interface is available!");
            Ok(true)
        } else {
            println!("The org.bluez.AgentManager1 interface is not available.");
            Ok(false)
        }
    }
    
    pub fn register_agent(session: &'a BluetoothSession,) -> Result<(), Box<dyn std::error::Error>> {
      
        // Connect to the system bus
        // let conn = Connection::get_private(dbus::BusType::System)?;
    
        // Define the BlueZ service and the interface
        let bluez_service = "org.bluez";
        let agent_path = "/org/bluez/agent";  // The agent object path
        let agent_manager_path = "/org/bluez/agent";  // AgentManager path
    
        // Send the RegisterAgent request to the AgentManager interface
        let msg = Message::new_method_call(bluez_service, agent_manager_path, AGENT_INTERFACE, REG_AGENT)?
            .append2(agent_path, "NoInputNoOutput");

    
        // Send the message and block until a reply is received
        let _response: Message = session.get_connection().send_with_reply_and_block(msg, 1000)?;
        
        if BluetoothAgent::check_agent_manager_available()? {
            println!("AgentManager available");
        }
        // Notify that the agent has been successfully registered
        println!("Agent registered successfully!");
        // while true {
        //     std::thread::sleep(Duration::from_secs(1));
        // }
        Ok(())
    }

}

//     fn register_agent(&self, conn: &Connection) -> Result<(), dbus::Error> {
//         // let proxy = dbus::blocking::Proxy::new("org.bluez", "/", Duration::from_millis(5000), conn);
//         match self.call_method("RegisterAgent", Some(&[MessageItem::Str("NoInputNoOutput".into())]), 5000) {
//             Ok(_) => println!("Agent registered"),
//             Err(e) => println!("Error registering agent: {:?}", e),
//         }
//         // self.call_method("org.bluez.AgentManager1", "RequestDefaultAgent", (self.path.clone(),));
//         Ok(())
//     }

//     fn unregister_agent(&self, conn: &Connection) -> Result<(), dbus::Error> {
//         // let proxy = dbus::blocking::Proxy::new("org.bluez", "/", Duration::from_millis(5000), conn);
//         // self.call_method("UnregisterAgent", "org.bluez.AgentManager1", "UnregisterAgent", (self.path.clone(),))?;
//         Ok(())
//     }
    // pub fn create_agent(session: &BluetoothSession, adapter: String) -> Result<(), Box<dyn std::error::Error>> {

    //     // let conn = Connection::get_private(dbus::BusType::System)?;
        
    //     let agent = BluetoothAgent::new(MANAGER_PATH.to_string(), adapter, &session);
    //     register_agent();
    //     // agent.register_agent(&agent.conn)?;
    
    //     // // Keep the program running to maintain the agent registered
    //     // loop {
    //     //     std::thread::sleep(Duration::from_secs(1));
    //     // }
    
    //     // agent.unregister_agent(&conn)?;
    //     Ok(())
    // }
//     fn call_method(
//         &self,
//         method: &str,
//         param: Option<&[MessageItem]>,
//         timeout_ms: i32,
//     ) -> Result<(), Box<dyn std::error::Error>> {
//         bluetooth_utils::call_method(
//             &self.session.get_connection(),
//             AGENT_INTERFACE,
//             &self.path,
//             method,
//             param,
//             timeout_ms,
//         )
//     }
//     // fn call_method(&self, method: &str, param: Option<[MessageItem; 1]>) -> Result<(), Box<Error>> {
//     //     let mut m = try!(Message::new_method_call(
//     //         SERVICE_NAME,
//     //         &self.adapter,
//     //         AGENT_INTERFACE,
//     //         method
//     //     ));
//     //     match param {
//     //         Some(p) => m.append_items(&p),
//     //         None => (),
//     //     };
//     //     try!(
//     //         self.session
//     //             .get_connection()
//     //             .send_with_reply_and_block(m, 1000)
//     //     );
//     //     Ok(())
//     // }
// }



// pub fn register_agent() -> Result<(), Box<dyn std::error::Error>> {
//     // Connect to the system bus (blocking)
//     let connection = Connection::get_private(BusType::System);
//     let connection2 = Connection::get_private(BusType::System);
//     let connection3 = Connection::get_private(BusType::System);
    
//     // Register the agent (here, you should include the full registration logic as before)
//     let msg = Message::new_method_call(
//         "org.bluez",
//         "/org/bluez",
//         "org.bluez.AgentManager1",
//         "RegisterAgent",
//     ).expect("Failed to create method call")
//     .append1("/org/bluez/agent");

//     connection.unwrap().send(msg).expect("Failed to send message");
//     let msg2 = Message::new_method_call(
//         "org.bluez",
//         "/org/bluez",
//         "org.bluez.AgentManager1",
//         "RequestDefaultAgent",
//     ).expect("Failed to create method call")
//     .append1("/org/bluez/agent");

//     connection2.unwrap().send(msg2).expect("Failed to send message");

//     Ok(())
//     // Update state to indicate that the agent is registered
//     // let mut state = state.lock().unwrap();
//     // state.registered = true;
// }

// extern crate dbus;

// use dbus::{Connection, Message, Path};
// use dbus::arg::RefArg;


// fn register_agent_test(&self, conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
//     // Register the agent interface on the DBus
//     let agent_path = Path::new("/org/bluez/agent")?;
//     conn.register_object_path(agent_path.clone(), self)?;

//     // Register agent with BlueZ
//     let bluez_agent_interface = "org.freedesktop.DBus.Properties";
//     conn.send(Message::new_method_call(
//         "org.bluez",
//         "/org/bluez",
//         bluez_agent_interface,
//         "RegisterAgent"
//     )?.append2(agent_path.clone(), "NoInputNoOutput"))?;

//     Ok(())
// }
// fn unregister_agent(state: &AgentState) {
//     // Connect to the system bus (blocking)
//     let connection = Connection::get_private(BusType::System);
    
//     // Unregister the agent (here, you should include the full unregistration logic)
//     let msg = Message::new_method_call(
//         "org.bluez",
//         "/org/bluez",
//         "org.freedesktop.DBus.Properties",
//         "Set",
//     ).expect("Failed to create method call")
//     .append2("org.bluez.AgentManager1", "Registered")
//     .append1(false);

//     connection.unwrap().send(msg).expect("Failed to send message");
    
//     // Update state to indicate that the agent is unregistered
//     // let mut state = state.lock().unwrap();
//     // state.registered = false;
// }
