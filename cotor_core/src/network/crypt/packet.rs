// use crate::network::packet::PacketData;
// 
// pub trait PacketEncrypter {
//     fn encrypt_packet(&self, packet: &PacketData) -> Result<Vec<u8>, &'static str>;
// }
// 
// pub trait PacketDecrypter {
//     fn decrypt_packet(&self, data: Vec<u8>) -> Result<PacketData, &'static str>;
// }