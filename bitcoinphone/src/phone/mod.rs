use portaudio as pa;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel, RecvError};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use portaudio::stream::{InputCallbackArgs, OutputCallbackArgs};
use std::convert::TryInto;
use portaudio::{OutputStreamSettings, InputStreamSettings, PortAudio, Stream, NonBlocking, Output, Input};
use std::collections::LinkedList;
use std::time::Duration;
use crate::util::constants::DataPacket;


pub struct PhoneConfig {
    pub(crate) sample_rate: f64,
    pub(crate) frames_per_buffer: u32,
    pub(crate) jitter_delay_nanos: u64,
}

pub struct Phone {
    config: PhoneConfig,
    data_sender: SyncSender<DataPacket>,
    phone_buffer: Mutex<LinkedList<(Vec<u8>, u32)>>,
}

impl Phone {
    pub fn new(config: PhoneConfig, data_sender: SyncSender<DataPacket>) -> SyncSender<DataPacket> {

        let mut phone = Arc::from(Phone{
            config,
            data_sender,
            phone_buffer:Mutex::new(LinkedList::new()),
        });

        println!("Starting up speaker!");
        let stream = phone.clone().spawn_streams();
        return phone.clone().spawn_receiver(stream.1, stream.0);
    }

    fn spawn_receiver(self: Arc<Self>, mut speaker: Stream<NonBlocking, Output<u8>>, mut mic: Stream<NonBlocking, Input<u8>>) -> SyncSender<DataPacket>{
        let (sender, receiver): (SyncSender<DataPacket>, Receiver<DataPacket>) = sync_channel(1000);

        println!("Spawning receiver");
        thread::spawn(move || {
            // Collect first packet and wait to adjust for jitter
            let (counter, buffer) = Self::parse_packet(receiver.recv().unwrap());


            self.clone().update_buffer(buffer, counter);
            thread::sleep(Duration::from_nanos(self.config.jitter_delay_nanos));

            // Run the speaker
            speaker.start();
            mic.time();

            // Run the loop to update the data.
            loop {
                match receiver.recv() {
                    Ok(DataPacket::Data{
                        counter,
                        buffer
                       }) => {
                        self.clone().update_buffer(buffer, counter);
                    }
                    Err(err) => panic!("Error {}", err),
                    _ => {
                        println!("Bad packet passed to phone! ignoring");
                    }
                }
            }
        });
        return sender;
    }

    fn parse_packet(packet: DataPacket) -> (u32, Vec<u8>) {
        return if let DataPacket::Data{counter, buffer} = packet {
            return (counter, buffer);
        } else {
            panic!("Invalid packet sent to phone.");
        }
    }

    fn update_buffer(self: Arc<Self>, packet_data: Vec<u8>, priority: u32) {
        let mut buffer = self.phone_buffer.lock()
            .expect("Unable to unlock buffer");
        // TODO: Implement algorithm to handle out-of-order messages
        buffer.push_back((packet_data, priority));
    }

    fn spawn_streams(self: Arc<Self>) -> (Stream<NonBlocking, Input<u8>>, Stream<NonBlocking, Output<u8>>) {
        let pa = pa::PortAudio::new()
            .expect("Unable to start portaudio");

        let mut output_settings = pa.default_output_stream_settings(
                1,
                self.config.sample_rate,
                self.config.frames_per_buffer
            ).expect("Unable to set output settings");
        output_settings.flags = pa::stream_flags::CLIP_OFF;

        let input_settings = pa.default_input_stream_settings(
                1,
                self.config.sample_rate,
                self.config.frames_per_buffer
            ).expect("Unable to set input settings");

        return (self.clone().create_receiver(input_settings, &pa), self.clone().create_sender(output_settings, &pa));
    }

    fn create_sender(self: Arc<Self>, settings: OutputStreamSettings<u8>, pa: &PortAudio) -> Stream<NonBlocking, Output<u8>> {
        let cloned_self = self.clone();
        let cb = move |pa::OutputStreamCallbackArgs{buffer, ..}| {

            let value = match cloned_self.phone_buffer.lock().unwrap().pop_front() {
                None => {
                    return pa::Continue;
                }
                Some(packet) => packet.0
            };

            buffer[..].clone_from_slice(value.as_slice());
            return pa::Continue;
        };

        let mut stream = pa
            .open_non_blocking_stream(settings, cb)
            .expect("Unable to start output stream");

        return stream;
    }

    fn create_receiver(self: Arc<Self>, settings: InputStreamSettings<u8>, pa: &PortAudio) -> Stream<NonBlocking, Input<u8>> {
        let cloned_self = self.clone();
        let mut counter = 0;
        let cb = move |pa::InputStreamCallbackArgs{ time,  buffer, ..}| {
            cloned_self.data_sender
                .send(DataPacket::Data{
                    counter,
                    buffer: buffer.to_vec(),
                })
                .unwrap();
            counter += 1;
            return pa::Continue;
        };

        let mut stream = pa
            .open_non_blocking_stream(settings, cb)
            .expect("Unable to start input stream");
        stream.start();
        return stream;
    }
}