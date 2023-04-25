use openssl::asn1::Asn1Time;
use openssl::bn::{BigNum, MsbOption};
use openssl::error::ErrorStack;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::x509::extension::{BasicConstraints, KeyUsage, SubjectKeyIdentifier};
use openssl::x509::{X509NameBuilder, X509};

fn generate_key_cert() -> Result<(X509, PKey<Private>), ErrorStack> {
    let rsa = Rsa::generate(2048)?;
    let key_pair = PKey::from_rsa(rsa)?;

    let mut x509_name = X509NameBuilder::new()?;
    x509_name.append_entry_by_text("C", "US")?;
    x509_name.append_entry_by_text("ST", "CA")?;
    x509_name.append_entry_by_text("O", "10.100.1.3")?;
    x509_name.append_entry_by_text("CN", "10.100.1.3")?;

    let x509_name = x509_name.build();

    let mut cert_builder = X509::builder()?;
    cert_builder.set_version(2)?;
    let serial_number = {
        let mut serial = BigNum::new()?;
        serial.rand(159, MsbOption::MAYBE_ZERO, false)?;
        serial.to_asn1_integer()?
    };
    cert_builder.set_serial_number(&serial_number)?;
    cert_builder.set_subject_name(&x509_name)?;
    cert_builder.set_issuer_name(&x509_name)?;
    cert_builder.set_pubkey(&key_pair)?;
    let not_before = Asn1Time::days_from_now(0)?;
    cert_builder.set_not_before(&not_before)?;
    let not_after = Asn1Time::days_from_now(365)?;
    cert_builder.set_not_after(&not_after)?;

    cert_builder.append_extension(BasicConstraints::new().critical().ca().build()?)?;
    cert_builder.append_extension(
        KeyUsage::new()
            .critical()
            .key_cert_sign()
            .crl_sign()
            .build()?,
    )?;

    let subject_key_identifier =
        SubjectKeyIdentifier::new().build(&cert_builder.x509v3_context(None, None))?;
    cert_builder.append_extension(subject_key_identifier)?;

    cert_builder.sign(&key_pair, MessageDigest::sha256())?;
    let cert = cert_builder.build();

    Ok((cert, key_pair))
}

#[cfg(test)]
mod server_test {

    extern crate openssl;

    use crate::{accept::accpet_connection, connect::connect};
    use rand::Rng;
    use std::{
        fs::{create_dir, File},
        io::{self, Write},
        net::ToSocketAddrs,
        path::Path,
        thread::{self, JoinHandle},
        time::Duration,
    };
    use tokio::runtime::Runtime;

    use super::generate_key_cert;

    #[test]
    fn run_server() -> io::Result<()> {
        let mut rng = rand::thread_rng();
        let rand = rng.gen::<u32>();

        let dir = format!("./test-dir-{}/", rand).to_string();
        let path = Path::new(&dir);

        create_dir(path)?;

        let (cert, key) = generate_key_cert().unwrap();

        let mut key_file = File::create(path.join("key.pem"))?;
        key_file.write_all(key.rsa().unwrap().private_key_to_pem().unwrap().as_ref())?;

        let mut cert_file = File::create(path.join("cert.pem"))?;
        cert_file.write_all(cert.to_pem().unwrap().as_ref())?;

        let mut threads: Vec<JoinHandle<()>> = Vec::new();
        let server_th = thread::spawn(move || {
            let dir = format!("./test-dir-{}/", rand).to_string();
            let path = Path::new(&dir);
            let rt = Runtime::new().unwrap();
            let addr = "0.0.0.0:5000".to_socket_addrs().unwrap().next();

            rt.block_on(accpet_connection(
                addr.unwrap(),
                true,
                Some(path.join("key.pem")),
                Some(path.join("cert.pem")),
            ));
        });

        threads.insert(0, server_th);

        thread::sleep(Duration::from_secs(1));

        let client_th = thread::spawn(move || {
            let dir = format!("./test-dir-{}/", rand).to_string();
            let path = Path::new(&dir);
            let rt = Runtime::new().unwrap();
            let addr = "10.100.1.3:5000".to_socket_addrs().unwrap().next();
            rt.block_on(connect(addr.unwrap(), Some(path.join("cert.pem"))));
        });

        threads.insert(1, client_th);

        for i in threads {
            i.join();
        }

        loop {
            thread::sleep(Duration::from_secs(1))
        }
        Ok(())
    }
}
