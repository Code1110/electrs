## Manual installation from source

**See below for automated/binary installation options.**

### Build dependencies

Note for Raspberry Pi 4 owners: the old versions of OS/toolchains produce broken binaries.
Make sure to use latest OS! (see #226)

Install [recent Rust](https://rustup.rs/) (1.41.1+, `apt install cargo` is preferred for Debian 10),
[latest Bitcoin Core](https://bitcoincore.org/en/download/) (0.16+)
and [latest Electrum wallet](https://electrum.org/#download) (3.3+).

Also, install the following packages (on Debian or Ubuntu):
```bash
$ sudo apt update
$ sudo apt install clang cmake build-essential  # for building 'rust-rocksdb'
```

There are two ways to compile `electrs`: by statically linking to `librocksdb` or dynamically linking.

The advantages of static linking:

* The binary is self-contained and doesn't need other dependencies, it can be transferred to other machine without worrying
* The binary should work pretty much with every common distro
* Different library installed elsewhere doesn't affect the behavior of `electrs`

The advantages of dynamic linking:

* If a (security) bug is found in the library, you only need to upgrade/recompile the library to fix it, no need to recompile `electrs`
* Updating rocksdb can be as simple as `apt upgrade`
* The build is significantly faster (if you already have the binary version of the library from packages)
* The build is deterministic
* Cross compilation is more reliable
* If another application is also using `rocksdb`, you don't store it on disk and in RAM twice

If you decided to use dynamic linking, you will also need to install the library.
On Debian:

```bash
$ sudo apt install librocksdb-dev
```

#### Preparing for cross compilation

Cross compilation can save you some time since you can compile `electrs` for a slower computer (like Raspberry Pi) on a faster machine
even with different CPU architecture.
Skip this if it's not your case.

If you want to cross-compile, you need to install some additional packages.
These cross compilation instructions use `aarch64`/`arm64` + Linux as an example.
(The resulting binary should work on RPi 4 with aarch64-enabled OS).
Change to your desired architecture/OS.

If you use Debian (or a derived distribution) you need to enable the target architecture:

```
$ sudo dpkg --add-architecture arm64
$ sudo apt update
```

If you use `cargo` from the repository

```bash
$ sudo apt install gcc-aarch64-linux-gnu gcc-aarch64-linux-gnu libc6-dev:arm64 libstd-rust-dev:arm64
```

If you use Rustup:

```bash
$ sudo apt install gcc-aarch64-linux-gnu gcc-aarch64-linux-gnu libc6-dev:arm64
$ rustup target add aarch64-unknown-linux-gnu
```

If you decided to use the system rocksdb (recommended if the target OS supports it), you need the version from the other architecture:

```bash
$ sudo apt install librocksdb-dev:arm64
```

#### Preparing man page generation (optional)

Optionally, you may install [`cfg_me`](https://github.com/Kixunil/cfg_me) tool for generating the manual page.
The easiest way is to run `cargo install cfg_me`.

#### Download electrs

```bash
$ git clone https://github.com/romanz/electrs
$ cd electrs
```

### Build

Note: you need to have enough free RAM to build `electrs`.
The build will fail otherwise.
Close those 100 old tabs in the browser. ;)

#### Static linking

First build should take ~20 minutes:
```bash
$ cargo build --locked --release
```

#### Dynamic linking

```
$ ROCKSDB_INCLUDE_DIR=/usr/include ROCKSDB_LIB_DIR=/usr/lib cargo build --locked --no-default-features --release
```

(Don't worry about `--no-default-features`, it's only related to rocksdb linking.)

#### Cross compilation

Run one of the commands above (depending on linking type) with argument `--target aarch64-unknown-linux-gnu` and prepended with env vars: `BINDGEN_EXTRA_CLANG_ARGS="-target gcc-aarch64-linux-gnu" RUSTFLAGS="-C linker=aarch64-linux-gnu-gcc"`

E.g. for dynamic linking case:

```
$ ROCKSDB_INCLUDE_DIR=/usr/include ROCKSDB_LIB_DIR=/usr/lib BINDGEN_EXTRA_CLANG_ARGS="-target gcc-aarch64-linux-gnu" RUSTFLAGS="-C linker=aarch64-linux-gnu-gcc" cargo build --locked --release --target aarch64-unknown-linux-gnu
```

It's a bit long but sufficient! You will find the resulting binary in `target/aarch64-unknown-linux-gnu/release/electrs` - copy it to your target machine.

#### Generating man pages

If you installed `cfg_me` to generate man page, you can run `cfg_me man` to see it right away or `cfg_me -o electrs.1 man` to save it into a file (`electrs.1`).

## Docker-based installation from source

Note: currently Docker installation links statically

```bash
$ docker build -t electrs-app .
$ mkdir db
$ docker run --network host \
             --volume $HOME/.bitcoin:/home/user/.bitcoin:ro \
             --volume $PWD/db:/home/user/db \
             --env ELECTRS_DB_DIR=/home/user/db \
             --rm -i -t electrs-app
```

## Native OS packages

There are currently no official/stable binary packages.

However, there's a [*beta* repository for Debian 10](https://deb.ln-ask.me) (should work on recent Ubuntu, but not tested well-enough)
The repository provides several significant advantages:

* Everything is completely automatic - after installing `electrs` via `apt`, it's running and will automatically run on reboot, restart after crash..
  It also connects to bitcoind out-of-the-box, no messing with config files or anything else.
  It just works.
* Prebuilt binaries save you a lot of time.
  The binary installation of all the components is under 3 minutes on common hardware.
  Building from source is much longer.
* The repository contains some seurity hardening out-of-the-box - separate users for services, use of [btc-rpc-proxy](https://github.com/Kixunil/btc-rpc-proxy), etc.

And two disadvantages:

* It's currently not trivial to independently verify the built packages, so you may need to trust the author of the repository.
  The build is now deterministic but nobody verified it independently yet.
* The repository is considered beta.
  electrs` seems to work well so far but was not tested heavily.
  The author of the repository is also a contributor to `electrs` and appreciates [bug reports](https://github.com/Kixunil/cryptoanarchy-deb-repo-builder/issues),
  [test reports](https://github.com/Kixunil/cryptoanarchy-deb-repo-builder/issues/61), and other contributions.

## Manual configuration

This applies only if you do **not** use some other automated systems such as Debian packages.
If you use automated systems, refer to their documentation first!

### Bitcoind configuration

Pruning must be turned **off** for `electrs` to work.
`txindex` is allowed but unnecessary for `electrs`.
However, you might still need it if you run other services (e.g.`eclair`)

The highly recommended way of authenticating `electrs` is using cookie file.
It's the most [secure](https://github.com/Kixunil/security_writings/blob/master/cookie_files.md) and robust method.
Set `rpccookiefile` option of `bitcoind` to a file within an existing directory which it can access.
You can skip it if you're running both daemons under the same user and with the default directories.

`electrs` will wait for `bitcoind` to sync, however, you will be unabe to use it until the syncing is done.

Example command for running `bitcoind` (assuming same user, default dirs):

```bash
$ bitcoind -server=1 -txindex=0 -prune=0
```
### Electrs configuration

Electrs can be configured using command line, environment variables and configuration files (or their combination).
It is highly recommended to use configuration files for any non-trivial setups since it's easier to manage.
If you're setting password manually instead of cookie files, configuration file is the only way to set it due to security reasons.

### Configuration files and priorities

The Toml-formatted config files ([an example here](config_example.toml)) are (from lowest priority to highest): `/etc/electrs/config.toml`, `~/.electrs/config.toml`, `./electrs.toml`.

The options in highest-priority config files override options set in lowest-priority config files.

**Environment variables** override options in config files and finally **arguments** override everythig else.

There are two special arguments `--conf` which reads the specified file and `--conf-dir`, which read all the files in the specified directory.

The options in those files override **everything** that was set previously, **including arguments** that were passed before these two special arguments.

In general, later arguments override previous ones.
It is a good practice to use these special arguments at the beginning of the command line in order to avoid confusion.

**Naming convention**

For each command line argument an **environment variable** of the same name with `ELECTRS_` prefix, upper case letters and underscores instead of hypens exists
(e.g. you can use `ELECTRS_ELECTRUM_RPC_ADDR` instead of `--electrum-rpc-addr`).

Similarly, for each such argument an option in config file exists with underscores instead of hypens (e.g. `electrum_rpc_addr`).

You need to use a number in config file if you want to increase verbosity (e.g. `verbose = 3` is equivalent to `-vvv`) and `true` value in case of flags (e.g. `timestamp = true`)

**Authentication**

In addition, config files support `auth` option to specify username and password.
This is not available using command line or environment variables for security reasons (other applications could read it otherwise).
**Important note**: `auth` is different from `cookie_file`, which points to a file containing the cookie instead of being the cookie itself!

If you are using `-rpcuser=USER` and `-rpcpassword=PASSWORD` of `bitcoind` for authentication, please use `auth="USER:PASSWORD"` option in one of the [config files](https://github.com/romanz/electrs/blob/master/doc/usage.md#configuration-files-and-priorities).
Otherwise, [`~/.bitcoin/.cookie`](https://github.com/bitcoin/bitcoin/blob/0212187fc624ea4a02fc99bc57ebd413499a9ee1/contrib/debian/examples/bitcoin.conf#L70-L72) will be used as the default cookie file,
allowing this server to use bitcoind JSONRPC interface.

Note: there was a `cookie` option in the version 0.8.7 and below, it's now deprecated - do **not** use, it will be removed.
Please read upgrade notes if you're upgrading to a newer version.

### Electrs usage

First index sync should take ~1.5 hours (on a dual core Intel CPU @ 3.3 GHz, 8 GB RAM, 1TB WD Blue HDD):
```bash
$ ./target/release/electrs -vvv --timestamp --db-dir ./db --electrum-rpc-addr="127.0.0.1:50001"
2018-08-17T18:27:42 - INFO - NetworkInfo { version: 179900, subversion: "/Satoshi:0.17.99/" }
2018-08-17T18:27:42 - INFO - BlockchainInfo { chain: "main", blocks: 537204, headers: 537204, bestblockhash: "0000000000000000002956768ca9421a8ddf4e53b1d81e429bd0125a383e3636", pruned: false, initialblockdownload: false }
2018-08-17T18:27:42 - DEBUG - opening DB at "./db/mainnet"
2018-08-17T18:27:42 - DEBUG - full compaction marker: None
2018-08-17T18:27:42 - INFO - listing block files at "/home/user/.bitcoin/blocks/blk*.dat"
2018-08-17T18:27:42 - INFO - indexing 1348 blk*.dat files
2018-08-17T18:27:42 - DEBUG - found 0 indexed blocks
2018-08-17T18:27:55 - DEBUG - applying 537205 new headers from height 0
2018-08-17T19:31:01 - DEBUG - no more blocks to index
2018-08-17T19:31:03 - DEBUG - no more blocks to index
2018-08-17T19:31:03 - DEBUG - last indexed block: best=0000000000000000002956768ca9421a8ddf4e53b1d81e429bd0125a383e3636 height=537204 @ 2018-08-17T15:24:02Z
2018-08-17T19:31:05 - DEBUG - opening DB at "./db/mainnet"
2018-08-17T19:31:06 - INFO - starting full compaction
2018-08-17T19:58:19 - INFO - finished full compaction
2018-08-17T19:58:19 - INFO - enabling auto-compactions
2018-08-17T19:58:19 - DEBUG - opening DB at "./db/mainnet"
2018-08-17T19:58:26 - DEBUG - applying 537205 new headers from height 0
2018-08-17T19:58:27 - DEBUG - downloading new block headers (537205 already indexed) from 000000000000000000150d26fcc38b8c3b71ae074028d1d50949ef5aa429da00
2018-08-17T19:58:27 - INFO - best=000000000000000000150d26fcc38b8c3b71ae074028d1d50949ef5aa429da00 height=537218 @ 2018-08-17T16:57:50Z (14 left to index)
2018-08-17T19:58:28 - DEBUG - applying 14 new headers from height 537205
2018-08-17T19:58:29 - INFO - RPC server running on 127.0.0.1:50001
```
You can specify options via command-line parameters, environment variables or using config files.
See the documentation above.

Note that the final DB size should be ~20% of the `blk*.dat` files, but it may increase to ~35% at the end of the inital sync (just before the [full compaction is invoked](https://github.com/facebook/rocksdb/wiki/Manual-Compaction)).

If initial sync fails due to `memory allocation of xxxxxxxx bytes failedAborted` errors, as may happen on devices with limited RAM, try the following arguments when starting `electrs`.
It should take roughly 18 hours to sync and compact the index on an ODROID-HC1 with 8 CPU cores @ 2GHz, 2GB RAM, and an SSD using the following command:

```bash
$ ./target/release/electrs -vvvv --index-batch-size=10 --jsonrpc-import --db-dir ./db --electrum-rpc-addr="127.0.0.1:50001"
```

The index database is stored here:
```bash
$ du db/
38G db/mainnet/
```

See below for [extra configuration suggestions](https://github.com/romanz/electrs/blob/master/doc/usage.md#extra-configuration-suggestions) that you might want to consider.

## Electrum client

If you happen to use the Electrum client from [the *beta* Debian repository](https://github.com/romanz/electrs/blob/master/doc/usage.md#cnative-os-packages), it's pre-configured out-of-the-box already
Read below otherwise.

There's a prepared script for launching `electrum` in such way to connect only to the local `electrs` instance to protect your privacy.

```bash
$ ./scripts/local-electrum.bash
+ ADDR=127.0.0.1
+ PORT=50001
+ PROTOCOL=t
+ electrum --oneserver --server=127.0.0.1:50001:t
<snip>
```

You can persist Electrum configuration (see `~/.electrum/config`) using:
```bash
$ electrum setconfig oneserver true
$ electrum setconfig server 127.0.0.1:50001:t
$ electrum   # will connect only to the local server
```

## Extra configuration suggestions

### SSL connection

In order to use a secure connection, you can also use [NGINX as an SSL endpoint](https://docs.nginx.com/nginx/admin-guide/security-controls/terminating-ssl-tcp/#)
by placing the following block in `nginx.conf`.

```nginx
stream {
        upstream electrs {
                server 127.0.0.1:50001;
        }

        server {
                listen 50002 ssl;
                proxy_pass electrs;

                ssl_certificate /path/to/example.crt;
                ssl_certificate_key /path/to/example.key;
                ssl_session_cache shared:SSL:1m;
                ssl_session_timeout 4h;
                ssl_protocols TLSv1 TLSv1.1 TLSv1.2 TLSv1.3;
                ssl_prefer_server_ciphers on;
        }
}
```

```bash
$ sudo systemctl restart nginx
$ electrum --oneserver --server=example:50002:s
```

Note: If you are connecting to electrs from Eclair Mobile or another similar client which does not allow self-signed SSL certificates, you can obtain a free SSL certificate as follows:

1. Follow the instructions at https://certbot.eff.org/ to install the certbot on your system.
2. When certbot obtains the SSL certificates for you, change the SSL paths in the nginx template above as follows:
```
ssl_certificate /etc/letsencrypt/live/<your-domain>/fullchain.pem;
ssl_certificate_key /etc/letsencrypt/live/<your-domain>/privkey.pem;
```

### Tor hidden service

Install Tor on your server and client machines (assuming Ubuntu/Debian):

```
$ sudo apt install tor
```

Add the following config to `/etc/tor/torrc`:
```
HiddenServiceDir /var/lib/tor/electrs_hidden_service/
HiddenServiceVersion 3
HiddenServicePort 50001 127.0.0.1:50001
```

If you use [the *beta* Debian repository](https://github.com/romanz/electrs/blob/master/doc/usage.md#cnative-os-packages),
it is cleaner to install `tor-hs-patch-config` using `apt` and then placing the configuration into a file inside `/etc/tor/hidden-services.d`.

Restart the service:
```
$ sudo systemctl restart tor
```

Note: your server's onion address is stored under:
```
$ sudo cat /var/lib/tor/electrs_hidden_service/hostname
<your-onion-address>.onion
```

On your client machine, run the following command (assuming Tor proxy service runs on port 9050):
```
$ electrum --oneserver --server <your-onion-address>.onion:50001:t --proxy socks5:127.0.0.1:9050
```

For more details, see http://docs.electrum.org/en/latest/tor.html.

### Sample Systemd Unit File

If you use [the *beta* Debian repository](https://github.com/romanz/electrs/blob/master/doc/usage.md#cnative-os-packages), you should skip this section,
as the appropriate systemd unit file is installed automatically.

You may wish to have systemd manage electrs so that it's "always on".
Here is a sample unit file (which assumes that the bitcoind unit file is `bitcoind.service`):

```
[Unit]
Description=Electrs
After=bitcoind.service

[Service]
WorkingDirectory=/home/bitcoin/electrs
ExecStart=/home/bitcoin/electrs/target/release/electrs --db-dir ./db --electrum-rpc-addr="127.0.0.1:50001"
User=bitcoin
Group=bitcoin
Type=simple
KillMode=process
TimeoutSec=60
Restart=always
RestartSec=60

[Install]
WantedBy=multi-user.target
```

## Monitoring

Indexing and serving metrics are exported via [Prometheus](https://github.com/pingcap/rust-prometheus):

```bash
$ sudo apt install prometheus
$ echo "
scrape_configs:
  - job_name: electrs
    static_configs:
      - targets: ['localhost:4224']
" | sudo tee -a /etc/prometheus/prometheus.yml
$ sudo systemctl restart prometheus
$ firefox 'http://localhost:9090/graph?g0.range_input=1h&g0.expr=index_height&g0.tab=0'
```

## RPC examples

You can invoke any supported RPC using `netcat`, for example:

```
$ echo '{"jsonrpc": "2.0", "method": "server.version", "params": ["", "1.4"], "id": 0}' | netcat 127.0.0.1 50001
{"id":0,"jsonrpc":"2.0","result":["electrs 0.8.6","1.4"]}
```

For more complex tasks, you may need to convert addresses to 
[script hashes](https://electrumx-spesmilo.readthedocs.io/en/latest/protocol-basics.html#script-hashes) - see 
[contrib/addr.py](https://github.com/romanz/electrs/blob/master/contrib/addr.py) for getting an address balance:

```
$ ./contrib/addr.py 144STc7gcb9XCp6t4hvrcUEKg9KemivsCR  # sample address from block #640699
144STc7gcb9XCp6t4hvrcUEKg9KemivsCR has {'confirmed': 12652436, 'unconfirmed': 0} satoshis
```

## Upgrading

> **If you're upgrading from version 0.8.7 to a higher version and used `cookie` option you should change your configuration!**
> The `cookie` option was deprecated and **will be removed eventually**!
> If you had actual cookie (from `~/bitcoin/.cookie` file) specified in `cookie` option, this was wrong as it wouldn't get updated when needed.
> It's strongly recommended to use proper cookie authentication using `cookie_file`.
> If you really have to use fixed username and password, explicitly specified in `bitcoind` config, use `auth` option instead.
> Users of `btc-rpc-proxy` using `public:public` need to use `auth` too.
> You can read [a detailed explanation of cookie deprecation with motivation explained](cookie_deprecation.md).

As with any other application, you need to remember how you installed `electrs` to upgrade it.
If you don't then here's a little help: run `which electrs` and compare the output

* If you got an error you didn't install `electrs` into your system in any way, it's probably sitting in the `target/release` directory of source
* If the path starts with `/bin/` then either you have used packaging system or you made a mistake the first time (non-packaged binaries must go to `/usr/local/bin`)
* If the path starts with `/usr/local/bin` you most likely copied electrs there after building
* If the path starts with `/home/YOUR_USERNAME/.cargo/bin` you most likely ran `cargo install`

### Upgrading distribution package

If you used Debian packaging system you only need this:

```
sudo apt update
sudo apt upgrade
```

Similarly for other distributions - use their respective commands.  
If a new version of `electrs` is not yet in the package system, try wait a few days or contact the maintainers of the packages if it's been a long time.

### Upgrading manual installation

1. Enter your `electrs` source directory, usually in `~/` but some people like to put it in something like `~/sources`.
   If you've deleted it, you need to `git clone` again.
2. `git checkout master`
3. `git pull`
4. Strongly recommended: `git verify-tag v0.8.6` (fix the version number if we've forgotten to update this docs ;)) should show "Good signature from 15C8 C357 4AE4 F1E2 5F3F 35C5 87CA E5FA 4691 7CBB"
5. `git checkout v0.8.6`
6. If you used static linking: `cargo build --locked --release`.
   If you used dynamic linking `ROCKSDB_INCLUDE_DIR=/usr/include ROCKSDB_LIB_DIR=/usr/lib cargo build --locked --no-default-features --release`.
   If you don't remember which linking you used, you probably used static.
   This step will take a few tens of minutes (but dynamic linking is a bit faster), go grab a coffee.
   Also remember that you need enough free RAM, the build will die otherwise
7. If you've previously copied `electrs` into `/usr/local/bin` run: sudo `cp target/release/electrs /usr/local/bin`
   If you've previously installed `electrs` using `cargo install`: `cargo install --locked --path . -f`
8. If you've manually configured systemd service: `sudo systemctl restart electrs`
