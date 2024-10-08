//!
//! `nbnet ddev` SubCommand
//!
//! The distributed version of `nbnet dev`.
//!

use crate::{
    cfg::{DDevCfg, DDevOp},
    common::*,
};
use chaindev::{
    beacon_ddev::{
        remote::{
            collect_files_from_nodes as env_collect_files,
            collect_tgz_from_nodes as env_collect_tgz, Remote,
        },
        Env, EnvCfg as SysCfg, EnvMeta, EnvOpts as SysOpts, HostAddr, Hosts, Node,
        NodeCmdGenerator, NodeKind, Op, NODE_HOME_GENESIS_DST, NODE_HOME_VCDATA_DST,
    },
    CustomOps, EnvName,
};
use ruc::*;
use serde::{Deserialize, Serialize};
use std::{env, fs, str::FromStr};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvCfg {
    sys_cfg: SysCfg<CustomInfo, Ports, ExtraOp>,
}

impl EnvCfg {
    pub fn exec(&self) -> Result<()> {
        self.sys_cfg.exec(CmdGenerator).c(d!()).map(|_| ())
    }
}

impl From<DDevCfg> for EnvCfg {
    fn from(cfg: DDevCfg) -> Self {
        let mut en = cfg
            .env_name
            .as_deref()
            .map(EnvName::from)
            .unwrap_or_default();

        let op = match cfg.op.unwrap_or_default() {
            DDevOp::Create(copts) => {
                if let Some(n) = copts.env_name {
                    en = n.into();
                }

                let hosts = copts
                    .hosts
                    .as_deref()
                    .map(|hs| hs.into())
                    .or_else(env_hosts);
                let hosts = pnk!(
                    hosts,
                    "No hosts registered! Use `--hosts` or $NBNET_DDEV_HOSTS to set."
                );

                let (genesis_tgz_path, genesis_vkeys_tgz_path) =
                    if let Some(s) = copts.genesis_data_pre_created {
                        let paths = s.split('+').collect::<Vec<_>>();
                        if 2 != paths.len() {
                            pnk!(Err(eg!("Invalid value")));
                        }
                        for p in paths.iter() {
                            if fs::metadata(p).is_err() {
                                pnk!(Err(eg!("File not accessible")));
                            }
                        }
                        (Some(paths[0].to_owned()), Some(paths[1].to_owned()))
                    } else {
                        (None, None)
                    };

                let custom_data = CustomInfo {
                    el_geth_bin: copts.el_geth_bin.unwrap_or("geth".to_owned()),
                    el_geth_extra_options: copts
                        .el_geth_extra_options
                        .unwrap_or_default(),
                    el_reth_bin: copts.el_reth_bin.unwrap_or("reth".to_owned()),
                    el_reth_extra_options: copts
                        .el_reth_extra_options
                        .unwrap_or_default(),
                    cl_bin: copts.cl_bin.unwrap_or_else(|| "lighthouse".to_owned()),
                    cl_extra_options: copts.cl_extra_options.unwrap_or_default(),
                };

                let envopts = SysOpts {
                    hosts,
                    block_itv: copts.block_time_secs.unwrap_or(0),
                    genesis_pre_settings: copts
                        .genesis_custom_settings_path
                        .unwrap_or_default(),
                    genesis_tgz_path,
                    genesis_vkeys_tgz_path,
                    initial_node_num: copts.initial_node_num,
                    initial_nodes_archive_mode: copts.initial_nodes_archive_mode,
                    custom_data,
                    force_create: copts.force_create,
                };

                Op::Create(envopts)
            }
            DDevOp::Destroy { env_name, force } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::Destroy(force)
            }
            DDevOp::DestroyAll { force } => Op::DestroyAll(force),
            DDevOp::Protect { env_name } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::Protect
            }
            DDevOp::Unprotect { env_name } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::Unprotect
            }
            DDevOp::Start { env_name, node_id } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::Start(node_id)
            }
            DDevOp::StartAll => Op::StartAll,
            DDevOp::Stop { env_name, node_id } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::Stop((node_id, false))
            }
            DDevOp::StopAll => Op::StopAll(false),
            DDevOp::PushNode {
                env_name,
                host_addr,
                is_reth,
                is_archive,
            } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::PushNode((
                    host_addr.map(|a| pnk!(HostAddr::from_str(&a))),
                    alt!(is_reth, RETH_MARK, GETH_MARK),
                    is_archive,
                ))
            }
            DDevOp::MigrateNode {
                env_name,
                node_id,
                host_addr,
            } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::MigrateNode((
                    node_id,
                    host_addr.map(|a| pnk!(HostAddr::from_str(&a))),
                ))
            }
            DDevOp::KickNode { env_name, node_id } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::KickNode(node_id)
            }
            DDevOp::PushHost { env_name, hosts } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                let hosts = pnk!(hosts.map(|h| h.into()).or_else(env_hosts));
                Op::PushHost(hosts)
            }
            DDevOp::KickHost {
                env_name,
                host_addr,
                force,
            } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::KickHost((pnk!(HostAddr::from_str(&host_addr)), force))
            }
            DDevOp::Show { env_name } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::Show
            }
            DDevOp::ShowAll => Op::ShowAll,
            DDevOp::List => Op::List,
            DDevOp::HostPutFile {
                env_name,
                local_path,
                remote_path,
                hosts,
            } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::HostPutFile {
                    local_path,
                    remote_path,
                    hosts: hosts.map(|h| h.into()).or_else(env_hosts),
                }
            }
            DDevOp::HostGetFile {
                env_name,
                remote_path,
                local_base_dir,
                hosts,
            } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::HostGetFile {
                    remote_path,
                    local_base_dir,
                    hosts: hosts.map(|h| h.into()).or_else(env_hosts),
                }
            }
            DDevOp::HostExec {
                env_name,
                cmd,
                script_path,
                hosts,
            } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::HostExec {
                    cmd,
                    script_path,
                    hosts: hosts.map(|h| h.into()).or_else(env_hosts),
                }
            }
            DDevOp::GetLogs {
                env_name,
                local_base_dir,
            } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::Custom(ExtraOp::GetLogs(local_base_dir))
            }
            DDevOp::DumpVcData {
                env_name,
                local_base_dir,
            } => {
                if let Some(n) = env_name {
                    en = n.into();
                }
                Op::Custom(ExtraOp::DumpVcData(local_base_dir))
            }
        };

        Self {
            sys_cfg: SysCfg { name: en, op },
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
struct CmdGenerator;

impl NodeCmdGenerator<Node<Ports>, EnvMeta<CustomInfo, Node<Ports>>> for CmdGenerator {
    fn cmd_cnt_running(
        &self,
        n: &Node<Ports>,
        e: &EnvMeta<CustomInfo, Node<Ports>>,
    ) -> String {
        format!(
            "ps ax -o pid,args | grep -E '({0}.*{3})|({1}.*{3})|({2}.*{3})' | grep -v 'grep' | wc -l",
            e.custom_data.el_geth_bin, e.custom_data.el_reth_bin, e.custom_data.cl_bin, n.home
        )
    }

    fn cmd_for_start(
        &self,
        n: &Node<Ports>,
        e: &EnvMeta<CustomInfo, Node<Ports>>,
    ) -> String {
        let mark = n.mark.unwrap_or(GETH_MARK);

        let home = &n.home;
        let genesis_dir = format!("{home}/genesis");
        let auth_jwt = format!("{home}/auth.jwt");

        pnk!(fs::write(&auth_jwt, ruc::algo::rand::rand_jwt()));

        let local_ip = &n.host.addr.local;
        let ext_ip = &n.host.addr.connection_addr();

        let online_nodes = e
            .nodes_should_be_online
            .iter()
            .copied()
            .take(5)
            .collect::<Vec<_>>();

        let (el_rpc_endpoints, cl_bn_rpc_endpoints): (Vec<_>, Vec<_>) = e
            .nodes
            .values()
            .chain(e.bootstraps.values())
            .filter(|n| online_nodes.contains(&n.id))
            .map(|n| {
                (
                    format!(
                        "http://{}:{}",
                        &n.host.addr.connection_addr(),
                        n.ports.el_rpc
                    ),
                    format!(
                        "http://{}:{}",
                        &n.host.addr.connection_addr(),
                        n.ports.cl_bn_rpc
                    ),
                )
            })
            .unzip();

        let prepare_cmd = format!(
            r#"
if [ ! -d {genesis_dir} ]; then
    tar -C {home} -xpf {home}/{NODE_HOME_GENESIS_DST} || exit 1
    mv {home}/$(tar -tf {home}/{NODE_HOME_VCDATA_DST} | head -1) {genesis_dir} || exit 1
fi "#
        );

        ////////////////////////////////////////////////
        // EL
        ////////////////////////////////////////////////

        let el_dir = format!("{home}/{EL_DIR}");
        let el_genesis = format!("{genesis_dir}/genesis.json");
        let el_discovery_port = n.ports.el_discovery;
        let el_discovery_v5_port = n.ports.el_discovery_v5;
        let el_rpc_port = n.ports.el_rpc;
        let el_rpc_ws_port = n.ports.el_rpc_ws;
        let el_engine_port = n.ports.el_engine_api;
        let el_metric_port = n.ports.el_metric;

        let el_rpc_endpoints = el_rpc_endpoints
            .iter()
            .map(|i| i.as_str())
            .collect::<Vec<_>>();

        let el_bootnodes =
            info!(el_get_boot_nodes(&el_rpc_endpoints)).unwrap_or_default();

        let el_cmd = if GETH_MARK == mark {
            let geth = &e.custom_data.el_geth_bin;

            let el_gc_mode = if matches!(n.kind, NodeKind::FullNode) {
                "full"
            } else {
                "archive" // Bootstrap nodes and Archive nodes
            };

            let cmd_init_part = format!(
                r#"
if [ ! -d {el_dir} ]; then
    {geth} init --datadir={el_dir} --state.scheme=hash \
        {el_genesis} >>{el_dir}/{EL_LOG_NAME} 2>&1 || exit 1
fi "#
            );

            let cmd_run_part_0 = format!(
                r#"
{geth} \
    --syncmode=full \
    --gcmode={el_gc_mode} \
    --networkid=$(grep -Po '(?<="chainId":)\s*\d+' {el_genesis} | tr -d ' ') \
    --datadir={el_dir} \
    --state.scheme=hash \
    --nat=extip:{ext_ip} \
    --discovery.port={el_discovery_port} \
    --discovery.v5 \
    --http --http.addr={local_ip} --http.port={el_rpc_port} --http.vhosts='*' --http.corsdomain='*' \
    --http.api='admin,debug,eth,net,txpool,web3,rpc' \
    --ws --ws.addr={local_ip} --ws.port={el_rpc_ws_port}--ws.origins='*' \
    --ws.api='admin,debug,eth,net,txpool,web3,rpc' \
    --authrpc.addr={local_ip} --authrpc.port={el_engine_port}\
    --authrpc.jwtsecret={auth_jwt} \
    --metrics \
    --metrics.port={el_metric_port} "#
            );

            let cmd_run_part_1 = if el_bootnodes.is_empty() {
                String::new()
            } else {
                format!(" --bootnodes='{el_bootnodes}'")
            };

            let cmd_run_part_2 = format!(" >>{el_dir}/{EL_LOG_NAME} 2>&1 &");

            cmd_init_part + &cmd_run_part_0 + &cmd_run_part_1 + &cmd_run_part_2
        } else if RETH_MARK == mark {
            let reth = &e.custom_data.el_reth_bin;

            let cmd_init_part = format!(
                r#"
if [ ! -d {el_dir} ]; then
    {reth} init --datadir={el_dir} --chain={el_genesis} \
        --log.file.directory={el_dir}/logs >>{el_dir}/{EL_LOG_NAME} 2>&1 || exit 1
fi "#
            );

            let cmd_run_part_0 = format!(
                r#"
{reth} node \
    --chain={el_genesis} \
    --datadir={el_dir} \
    --log.file.directory={el_dir}/logs \
    --ipcdisable \
    --nat=extip:{ext_ip} \
    --discovery.port={el_discovery_port} \
    --enable-discv5-discovery
    --discovery.v5.port={el_discovery_v5_port} \
    --http --http.addr={local_ip} --http.port={el_rpc_port} --http.corsdomain='*' \
    --http.api='admin,debug,eth,net,txpool,web3,rpc' \
    --ws --ws.addr={local_ip} --ws.port={el_rpc_ws_port}--ws.origins='*' \
    --ws.api='admin,debug,eth,net,txpool,web3,rpc' \
    --authrpc.addr={local_ip} --authrpc.port={el_engine_port}\
    --authrpc.jwtsecret={auth_jwt} \
    --metrics='{ext_ip}:{el_metric_port}' "#
            );

            let mut cmd_run_part_1 = if el_bootnodes.is_empty() {
                String::new()
            } else {
                format!(" --bootnodes='{el_bootnodes}' --trusted-peers='{el_bootnodes}'")
            };

            if matches!(n.kind, NodeKind::FullNode) {
                cmd_run_part_1.push_str(" --full");
            }

            let cmd_run_part_2 = format!(" >>{el_dir}/{EL_LOG_NAME} 2>&1 &");

            cmd_init_part + &cmd_run_part_0 + &cmd_run_part_1 + &cmd_run_part_2
        } else {
            pnk!(Err(eg!("The fucking world is over!")))
        };

        ////////////////////////////////////////////////
        // CL
        ////////////////////////////////////////////////

        let lighthouse = &e.custom_data.cl_bin;

        let cl_bn_dir = format!("{home}/{CL_BN_DIR}");
        let cl_vc_dir = format!("{home}/{CL_VC_DIR}");
        let cl_genesis = genesis_dir;
        let cl_bn_discovery_port = n.ports.cl_discovery;
        let cl_bn_discovery_quic_port = n.ports.cl_discovery_quic;
        let cl_bn_rpc_port = n.ports.cl_bn_rpc;
        let cl_vc_rpc_port = n.ports.cl_vc_rpc;
        let cl_bn_metric_port = n.ports.cl_bn_metric;
        let cl_vc_metric_port = n.ports.cl_vc_metric;

        let cl_slots_per_rp = if matches!(n.kind, NodeKind::FullNode) {
            2048
        } else {
            32
        };

        let cl_bn_rpc_endpoints = cl_bn_rpc_endpoints
            .iter()
            .map(|i| i.as_str())
            .collect::<Vec<_>>();

        let (cl_bn_bootnodes, cl_bn_trusted_peers) =
            info!(cl_get_boot_nodes(&cl_bn_rpc_endpoints)).unwrap_or_default();

        // For `ddev`, should change to `n.host.addr`
        let checkpoint_sync_url = e
            .nodes_should_be_online
            .iter()
            .take(1)
            .flat_map(|n| e.bootstraps.get(n).or_else(|| e.nodes.get(n)))
            .next()
            .map(|n| format!("http://{ext_ip}:{}", n.ports.cl_bn_rpc));

        let cl_bn_cmd = {
            let cmd_run_part_0 = format!(
                r#" (sleep 1;
{lighthouse} beacon_node \
    --testnet-dir={cl_genesis} \
    --datadir={cl_bn_dir} \
    --staking \
    --slots-per-restore-point={cl_slots_per_rp} \
    --enr-address={ext_ip} \
    --disable-enr-auto-update \
    --disable-upnp \
    --listen-address={local_ip} \
    --port={cl_bn_discovery_port} \
    --discovery-port={cl_bn_discovery_port} \
    --quic-port={cl_bn_discovery_quic_port}\
    --execution-endpoints='http://{local_ip}:{el_engine_port}' \
    --jwt-secrets={auth_jwt} \
    --suggested-fee-recipient={FEE_RECIPIENT} \
    --http --http-address={local_ip} \
    --http-port={cl_bn_rpc_port} --http-allow-origin='*' \
    --metrics --metrics-address={local_ip} \
    --metrics-port={cl_bn_metric_port} --metrics-allow-origin='*' "#
            );

            let mut cmd_run_part_1 = if cl_bn_bootnodes.is_empty() {
                String::new()
            } else {
                format!(" --boot-nodes='{cl_bn_bootnodes}' --trusted-peers='{cl_bn_trusted_peers}'")
            };

            if let Some(url) = checkpoint_sync_url {
                cmd_run_part_1.push_str(&format!(" --checkpoint-sync-url={url}"));
            }

            let cmd_run_part_2 = format!(" >>{cl_bn_dir}/{CL_BN_LOG_NAME} 2>&1) &");

            cmd_run_part_0 + &cmd_run_part_1 + &cmd_run_part_2
        };

        let cl_vc_cmd = {
            let beacon_nodes = format!("http://{local_ip}:{}", n.ports.cl_bn_rpc);

            let cmd_run_part_0 = if n.id == *e.bootstraps.keys().next().unwrap() {
                // The first bootstrap node
                format!(
                    r#"
                if [[ ! -d '{cl_vc_dir}/validators' ]]; then
                    mkdir -p {cl_vc_dir} || exit 1
                    tar -C {cl_vc_dir} -xpf {home}/{NODE_HOME_VCDATA_DST} || exit 1
                    mv $(tar -tf {home}/{NODE_HOME_VCDATA_DST} | head -1)/* ./ || exit 1
                fi "#
                )
            } else {
                String::new()
            };

            let cmd_run_part_1 = format!(
                r#"(sleep 2;
{lighthouse}/lighthouse validator_client \
    --testnet-dir={cl_genesis} \
    --datadir={cl_vc_dir}\
    --beacon-nodes='{beacon_nodes} \
    --init-slashing-protection \
    --suggested-fee-recipient={FEE_RECIPIENT} \
    --unencrypted-http-transport \
    --http --http-address={ext_ip} \
    --http-port={cl_vc_rpc_port} --http-allow-origin='*' \
    --metrics --metrics-address={ext_ip} \
    --metrics-port={cl_vc_metric_port} --metrics-allow-origin='*' \
     >>{cl_vc_dir}/{CL_VC_LOG_NAME} 2>&1) &
     "#
            );

            cmd_run_part_0 + &cmd_run_part_1
        };

        ////////////////////////////////////////////////
        // FINAL
        ////////////////////////////////////////////////

        format!(
            r#"
            {prepare_cmd}
            {el_cmd}
            {cl_bn_cmd}
            {cl_vc_cmd} "#
        )
    }

    fn cmd_for_stop(
        &self,
        n: &Node<Ports>,
        _e: &EnvMeta<CustomInfo, Node<Ports>>,
        force: bool,
    ) -> String {
        format!(
            "for i in \
            $(ps ax -o pid,args|grep '{}'|sed -r 's/(^ *)|( +)/ /g'|cut -d ' ' -f 2); \
            do kill {} $i; done",
            &n.home,
            alt!(force, "-9", ""),
        )
    }

    fn cmd_for_migrate(
        &self,
        src: &Node<Ports>,
        dst: &Node<Ports>,
        _e: &EnvMeta<CustomInfo, Node<Ports>>,
    ) -> impl FnOnce() -> Result<()> {
        || {
            let tgz = format!("vcdata_{}.tar.gz", ts!());

            let src_cmd = format!("tar -C /tmp -zcf {tgz} {}/{CL_VC_DIR}", &src.home);
            let src_remote = Remote::from(&src.host);
            src_remote
                .exec_cmd(&src_cmd)
                .c(d!())
                .and_then(|_| src_remote.get_file("/tmp/{tgz}", "/tmp/{tgz}").c(d!()))?;

            let dst_cmd = format!(
                "rm -rf {0}/{CL_VC_DIR} && tar -C {0} -zcf /tmp/{tgz}",
                &src.home
            );
            let dst_remote = Remote::from(&dst.host);
            dst_remote
                .put_file("/tmp/{tgz}", "/tmp/{tgz}")
                .c(d!())
                .and_then(|_| dst_remote.exec_cmd(&dst_cmd).c(d!()))
                .map(|_| ())
        }
    }
}

//////////////////////////////////////////////////
//////////////////////////////////////////////////

fn env_hosts() -> Option<Hosts> {
    env::var("NBNET_DDEV_HOSTS")
        .c(d!())
        .map(|s| Hosts::from(&s))
        .ok()
}

//////////////////////////////////////////////////
//////////////////////////////////////////////////

#[derive(Clone, Debug, Serialize, Deserialize)]
enum ExtraOp {
    GetLogs(Option<String>),
    DumpVcData(Option<String>),
}

impl CustomOps for ExtraOp {
    fn exec(&self, en: &EnvName) -> Result<()> {
        let env = Env::<CustomInfo, Ports, CmdGenerator>::load_env_by_name(en)
            .c(d!())?
            .c(d!("ENV does not exist!"))?;

        match self {
            Self::GetLogs(ldir) => env_collect_files(
                &env,
                &[
                    "{EL_DIR}/{EL_LOG_NAME}",
                    "{CL_BN_DIR}/{CL_BN_LOG_NAME}",
                    "{CL_VC_DIR}/{CL_VC_LOG_NAME}",
                ],
                ldir.as_deref(),
            )
            .c(d!()),
            Self::DumpVcData(ldir) => {
                env_collect_tgz(&env, &["{CL_VC_DIR}"], ldir.as_deref()).c(d!())
            }
        }
    }
}
