#![cfg(feature = "yozuk-skill-qrcode")]

mod common;
use common::cmd;
use yozuk_sdk::prelude::*;

#[test]
fn encode() {
    assert_eq!(
        cmd(tk!(["Hello World!", "to", "QRCode"])),
        Some(CommandArgs::new().add_args(["yozuk-skill-qrcode", "Hello World!"]))
    );
    assert_eq!(
        cmd(tk!(["😍😗😋", "to", "QR"])),
        Some(CommandArgs::new().add_args(["yozuk-skill-qrcode", "😍😗😋"]))
    );
    assert_eq!(
        cmd(tk!([
            "quick brown fox jumps over the lazy dog",
            "to",
            "qrcode"
        ])),
        Some(CommandArgs::new().add_args([
            "yozuk-skill-qrcode",
            "quick brown fox jumps over the lazy dog"
        ]))
    );
    assert_eq!(
        cmd(tk!(["2beae68d34cd6504bbe8e798b6a00a26", "to", "qr"])),
        Some(
            CommandArgs::new().add_args(["yozuk-skill-qrcode", "2beae68d34cd6504bbe8e798b6a00a26"])
        )
    );
}
