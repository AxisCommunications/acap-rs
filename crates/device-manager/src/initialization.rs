//! Procedures for taking a restored device to some useful state.
use std::{
    io::{BufRead, BufReader},
    time::Duration,
};

use acap_vapix::{parameter_management, systemready, HttpClient};
use log::{debug, info};
use tokio::time::sleep;
use url::{Host, Url};

use crate::vapix::{
    axis_cgi::{self, pwdgrp},
    config,
};

// TODO: Remove asserts that could be controlled by server

fn log_stdout(mut cmd: std::process::Command) -> anyhow::Result<()> {
    cmd.stdout(std::process::Stdio::piped());

    debug!("Spawning child {cmd:#?}...");
    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().unwrap();

    let lines = BufReader::new(stdout).lines();
    for line in lines {
        let line = line?;
        if !line.is_empty() {
            debug!("Child said {:?}.", line);
        }
    }

    debug!("Waiting for child...");
    let status = child.wait()?;
    if !status.success() {
        debug!("Child exited with status {status:?}");
    }
    Ok(())
}

async fn restore_root_ssh_user(client: &HttpClient, pass: &str) -> anyhow::Result<()> {
    info!("Unsetting restrictRootAccess...");
    axis_cgi::featureflag1::set(
        client,
        vec![("restrictRootAccess".to_string(), false)]
            .into_iter()
            .collect(),
    )
    .await?;

    info!("Resetting root password...");
    config::ssh1::update_user(client, "root", pass).await?;
    Ok(())
}

async fn wait_for_param(client: &HttpClient, key: &str, value: &str) -> anyhow::Result<()> {
    loop {
        match parameter_management::list()
            .group(key)
            .execute(client)
            .await
        {
            Ok(kvps) => {
                debug!("Presumed changed");
                assert_eq!(kvps.get(key).unwrap(), value);
                return Ok(());
            }
            Err(e) => {
                debug!("Presumed unchanged because {e}");
                sleep(Duration::from_secs(1)).await;
                continue;
            }
        }
    }
}

pub async fn initialize(host: Host, pass: &str) -> anyhow::Result<HttpClient> {
    let primary_user = "root";
    let no_auth_client = HttpClient::new(Url::parse(&format!("http://{host}")).unwrap());

    debug!("Assert that device can be adopted...");
    assert!(systemready::systemready()
        .execute(&no_auth_client)
        .await?
        .need_setup());

    info!("Adding the primary user...");
    pwdgrp::add(
        &no_auth_client,
        primary_user,
        pass,
        pwdgrp::Group::Root,
        false,
        pwdgrp::Role::AdminOperatorViewerPtz,
    )
    .await?;

    let digest_auth_client = no_auth_client.digest_auth(primary_user, pass);
    wait_for_param(
        &digest_auth_client,
        "root.Properties.API.Browser.RootPwdSetValue",
        "yes",
    )
    .await?;

    info!("Adding other users...");
    pwdgrp::add(
        &digest_auth_client,
        "ariel",
        pass,
        pwdgrp::Group::Users,
        true,
        pwdgrp::Role::AdminOperatorViewerPtz,
    )
    .await?;
    pwdgrp::add(
        &digest_auth_client,
        "orion",
        pass,
        pwdgrp::Group::Users,
        true,
        pwdgrp::Role::OperatorViewer,
    )
    .await?;
    pwdgrp::add(
        &digest_auth_client,
        "vega",
        pass,
        pwdgrp::Group::Users,
        true,
        pwdgrp::Role::Viewer,
    )
    .await?;

    info!("Enabling SSH...");
    parameter_management::update()
        .set("root.Network.SSH.Enabled", "yes")
        .execute(&digest_auth_client)
        .await?;

    // TODO: Consider factoring out to `acap-ssh-utils` crate.
    info!("Removing device from known_hosts...");
    let mut ssh_keygen = std::process::Command::new("ssh-keygen");
    ssh_keygen.arg("-R").arg(host.to_string());
    log_stdout(ssh_keygen)?;

    // TODO: Check firmware version, make this call only when needed, and fail failures.
    if let Err(e) = restore_root_ssh_user(&digest_auth_client, pass).await {
        info!("Could not restore root ssh user because {e} (this is expected on older firmware)");
    }

    // TODO: Capture stderr
    info!("Copying SSH key...");
    let mut sshpass = std::process::Command::new("sshpass");
    sshpass
        .arg(format!("-p{}", pass))
        .arg("ssh-copy-id")
        .args(["-o", "PubkeyAuthentication=no"])
        .args(["-o", "StrictHostKeyChecking=no"])
        .arg(&format!("root@{}", host));
    log_stdout(sshpass)?;

    Ok(digest_auth_client)
}
