# csync

csync is a simple network-based cross-device clipboard synchronization tool. It facilitates clipboard synchronization among different platforms (now support MacOS, Linux, and Windows). csync does not require direct network communication between these devices but mandates their ability to access a shared csyncd server for data exchange.

To ensure privacy, csync supports encrypting your clipboard data with a password. For details on encryption and security considerations, please refer to [SECURE.md](SECURE.md). It is strongly recommended to carefully read its contents before using csync.

## Install csyncd

You need to prepare a server to deploy csyncd, which is used for exchanging clipboard data between different devices. All devices should be able to access this server.

Somethings to know before your starting:

- If you deploy csyncd on public, clipboard data will be transmitted over the public network. Make sure csyncd is configured with a password to prevent privacy leaks.
- csyncd does not store data on the disk but rather in memory. If you restart csyncd, some unsynchronized data may be lost.
- csyncd does not support distributed deployment and can only be deployed as a single instance.

csyncd exposes its service on port 7703. I provide various deployment methods, with Docker being the recommended choice.

<details>
<summary>Docker</summary>

Run the command:

```bash
docker run -e 7703:7703 fioncat/csyncd:latest --password <your-password>
```

</details>

<details>
<summary>k8s</summary>

You can apply [csyncd/k8s.yml](csyncd/k8s.yml) file.

Download the yaml file:

```bash
curl -sSL https://raw.githubusercontent.com/fioncat/csync/main/csyncd/k8s.yml > csyncd-k8s.yml
```

Replace the default password:

```bash
sed -i 's/test123/<your-password>' csyncd-k8s.yml
```

Deploy to k8s:

```bash
kubectl -n <your-namespace> apply -f csyncd-k8s.yml
```

We use [Service](https://kubernetes.io/docs/concepts/services-networking/service/) to expose csyncd, you can use k8s DNS address `<namespace>.csyncd.<cluster>`, or [ClusterIP](https://kubernetes.io/docs/concepts/services-networking/cluster-ip-allocation/) to access csyncd.

By default, you can only access csyncd within the cluster. If you wish to expose the service externally, you may need to use [Ingress](https://kubernetes.io/docs/concepts/services-networking/ingress/) or [LoadBalancer Service](https://kubernetes.io/docs/tasks/access-application-cluster/create-external-load-balancer/).

</details>

<details>
<summary>systemd</summary>

The most traditional deployment method, suitable for various Linux distributions that support systemd.

First, download the latest version of the csyncd binary file from the [release page](https://github.com/fioncat/csync/releases/latest) and copy the file to `/usr/local/bin`.

Use the command to ensure the csyncd version:

```bash
csyncd --build-info
```

Download systemd service file:

```bash
curl -sSL https://raw.githubusercontent.com/fioncat/csync/main/csyncd/csyncd.service > /lib/systemd/system/csyncd.service
```

Replace password:

```bash
sed -i 's/test123/<your-password>' /lib/systemd/system/csyncd.service
```

Start the serivce:

```bash
systemctl daemon-reload
systemctl start csyncd
```

</details>


## Install csync

csync is used to interact with csyncd to retrieve clipboard data from other devices and synchronize the data to the clipboard of the current device.

csync operates on a publish/subscribe model. It can publish the clipboard of the current device to a channel or subscribe to the clipboard of other devices on a specific channel.

For example, consider two devices, `a` and `b`. Device `a` publishes its clipboard to `channel a` and subscribes to `channel b`; device `b` publishes its clipboard to `channel b` and subscribes to `channel a`. This way, clipboard sharing is achieved between the two devices.

If you have more devices, you can share the clipboard on additional channels and simultaneously subscribe to multiple channels.

You can even choose to only publish, not subscribe, to achieve `write_only`; or only subscribe, not publish, to achieve `read_only`.

Please download csync from the [release page](https://github.com/fioncat/csync/releases/latest).

The basic usage:

```bash
csync <publish-name>@<csyncd-addr>/<sub-0>,<sub-1>,...
```

Arguments:

- `<publish-name>`: The channel name to publish current device's clipboard.
- `<csyncd-addr>`: The csyncd address (not include port, if you want to set port, please use `-p` flag).
- `<sub0>,<sub-1>,...`: The subcribe channel(s), can be multiple.

If your csyncd is configured with a password, before initiating synchronization, csync will prompt you to enter the password for authentication.

## Build from source

You can build csync from source, this require Rust 1.75+ installed.

If you are using Linux without `x11`, you might need to install [xcb](https://xcb.freedesktop.org/) manually:

<details>
<summary>ArchLinux</summary>

```bash
sudo pacman -S libxcb lib32-libxcb xcb-util	lib32-xcb-util
```

</details>

<details>
<summary>Ubuntu</summary>

```bash
sudo apt-get install libx11-xcb-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

</details>

Build all:

```bash
git clone https://github.com/fioncat/csync.git /path/to/csync
cd /path/to/csync
cargo build --release --locked
```

Install csync:

```bash
cargo install --git https://github.com/fioncat/csync csync
```

Install csyncd:

```bash
cargo install --git https://github.com/fioncat/csync csyncd
```