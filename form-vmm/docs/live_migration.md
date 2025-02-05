# Live Migration

This document gives two examples of how to use the live migration
support in Cloud Hypervisor:

1. local migration - migrating a VM from one Cloud Hypervisor instance to another on the same machine;
1. remote migration - migrating a VM between two machines;

> :warning: These examples place sockets /tmp. This is done for
> simplicity and should not be done in production.

## Local Migration (Suitable for Live Upgrade of VMM)

Launch the source VM (on the host machine):

```console
$ target/release/cloud-hypervisor
    --kernel ~/workloads/vmlinux \
    --disk path=~/workloads/focal.raw \
    --cpus boot=1 --memory size=1G,shared=on \
    --cmdline "root=/dev/vda1 console=ttyS0"  \
    --serial tty --console off --api-socket=/tmp/api1
```

Launch the destination VM from the same directory (on the host machine):

```console
$ target/release/cloud-hypervisor --api-socket=/tmp/api2
```

Get ready for receiving migration for the destination VM (on the host machine):

```console
$ target/release/ch-remote --api-socket=/tmp/api2 receive-migration unix:/tmp/sock
```

Start to send migration for the source VM (on the host machine):

```console
$ target/release/ch-remote --api-socket=/tmp/api1 send-migration --local unix:/tmp/sock
```

When the above commands completed, the source VM should be successfully
migrated to the destination VM. Now the destination VM is running while
the source VM is terminated gracefully.

## Remote Migration

In this example, we will migrate a VM from one machine (`src`) to
another (`dst`) across the network. To keep it simple, we will use a
minimal VM setup without storage.

Because Cloud Hypervisor does not natively support migrating via TCP
connections, we will tunnel traffic through `socat`.

### Preparation

Make sure that `src` and `dst` can reach each other via the
network. You should be able to ping each machine. Also each machine
should have an open TCP port. For this example we assume port 6000.

You will need a kernel and initramfs for a minimal Linux system. For
this example, we will use the Debian netboot image.

Place the kernel and initramfs into the _same directory_ on both
machines. This is important for the migration to succeed. We will use
`/var/images`:

```console
src $ export DEBIAN=https://ftp.debian.org/debian/dists/stable/main/installer-amd64/current/images/netboot/debian-installer/amd64
src $ mkdir -p /var/images
src $ curl $DEBIAN/linux > /var/images/linux
src $ curl $DEBIAN/initrd.gz > /var/images/initrd
```

Repeat the above steps on the destination host.

### Starting the Receiver VM

On the receiver side, we prepare an empty VM:

```console
dst $ cloud-hypervisor --api-socket /tmp/api
```

In a different terminal, configure the VM as a migration target:

```console
dst $ ch-remote --api-socket=/tmp/api receive-migration unix:/tmp/sock
```

In yet another terminal, forward TCP connections to the Unix domain socket:

```console
dst $ socat TCP-LISTEN:6000,reuseaddr UNIX-CLIENT:/tmp/sock
```

### Starting the Sender VM

Let's start the VM on the source machine:

```console
src $ cloud-hypervisor \
        --serial tty --console off \
        --cpus boot=2 --memory size=4G \
        --kernel /var/images/linux \
        --initramfs /var/images/initrd \
        --cmdline "console=ttyS0" \
        --api-socket /tmp/api
```

After a few seconds the VM should be up and you can interact with it.

### Performing the Migration

First, we start `socat`:

```console
src $ socat UNIX-LISTEN:/tmp/sock,reuseaddr TCP:dst:6000
```

Then we kick-off the migration itself:

```console
src $ ch-remote --api-socket=/tmp/api send-migration unix:/tmp/sock
```

When the above commands completed, the VM should be successfully
migrated to the destination machine without interrupting the workload.
