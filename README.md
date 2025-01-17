# Formation 

#### A public, decentralized, verfiable, and self-replicating protocol for trustless, confidential Virtual private servers, coordinating as a Fog Compute network to power the Age of Autonomy.

<hr>

## Contributing to Formation

### WARNING! 
<hr>

<p> This project is still very early in its development. While it is inching closer to being production ready, there are no guarantees made. Please report issues here in the primary repository.
</p>

## Running a Node
<hr>

There are a few different ways that you can run a Formation node and participate in the network. For full documentation see our [Official Docs](docs.formation.cloud), and navigate to the **Operators** section.

The easiest way to get started is to simply run our Docker image in privileged mode with --network=host.

```bash
docker run --privileged --network=host --device=/dev/kvm \
    -v /var/run/docker.sock:/var/run/docker.sock -dit formation:latest
```

The **Formation** docker image requires that it be run in *privileged* mode, and while privileged mode is outside the scope of this particular document, we highly suggest you take the time to understand the implications of such. 

It also requires that you provide the `kvm` device to the container, as the Formation Virtual Machine Manager, Monitors and Hypervisor relies on KVM under the hood. Lastly, for now, we highly suggest your run it with the host network. The way that Formation provisions developers secure, confidential access to their instances is over a private VPN tunnel mesh network that runs wireguard under the hood. Configuring public access to the mesh network over the docker bridge network is still experimental, and you are likely to run into some headaches as a result. If you're looking to contribute to the project, and have expertise in container networking, linux networking, and would like to help make this process simpler so that the Formation node image can run without the host network access, please see the **Contributing to Formation** section above.

Running the image as described above will bootstrap you into an unofficial developer network. To join the official devnet please join our discord, navigate to the **Operators channel** and reach out to the core team there for more information on how to participate. 

##### Run Single Node Local Test
<hr>
<hr>

##### Run Multinode Local Test 

<hr>
<hr>

##### Join Official Developer Network

<hr>
<hr>

## Initializing a Developer Kit

This is an optional step that will save you a lot of time and effort. The rest of the **Deploying an App** guide in here assumes you complete this step. If you choose not to complete this step, please see the [Official Docs](docs.formation.cloud) for CLI requirements, or use our [web-based deployments UI](dev.formation.cloud). 

From anywhere in your terminal run the following command:

```bash
form kit init
```

This will launch the init wizard. If you choose not to run the interactive wizard and simply answer each prompt to build your Formation dev kit, you can see our [Official Docs] for a list of CLI args or run `form kit --help` to see the optional and required arguments for the Formation dev kit.

## Deploying an App

```bash
cd /path/to/app/root
```

##### Build a `Formfile`
`Formfile` is the Formation networks equivalent to a `Dockerfile`, and they are intentionally very similar in syntax and purpose. 

Where `Dockerfile` defines a manifest or recipe for deterministically building a container image, Formfile does the same thing for both building and configuring a Formation VPS instance. Given the differences between containers and Linux Virtual Machines, there are some differences in the options and commands that you have at your disposal.

Below is an example of a simple `Formfile` that defines a minimal VPS instance for hosting a simple `hello-world` http server built in `python`. For full documentation see [Formfile Docs](docs.formation.cloud/formfile)

```Dockerfile
NAME hello-server

USER username:bigdog passwd:bigdog123 sudo:true

VCPU 1

MEM 512

DISK 5

COPY ./hello.py /app

INSTALL python3

WORKDIR /app

ENTRYPOINT ["python3", "/app/hello.py"]
```

##### Build a Formpack

After you have defined your `Formfile` in the project root, you can use the `form` CLI to validate your `Formfile` 

```bash
form pack validate .
```
 Once validated, you can use the same CLI to request your app be built into a `Formpack`

```bash
form pack build .
```

This will package together your application artifacts based on the `Formfile`. If the `Formfile` is absent of any `COPY` instructions, the `Formpack` builder assumes that you want the entire directory (excluding the `Formfile`) to be include in your instance. To avoid this (a scenario where you will be cloning your application repo from github for example), you must have a `NOCOPY` command on one of the lines before the `ENTRYPOINT` command in the `Formfile`

Once your `Formpack` has successfully been built (i.e. you receive an API response with a `Formpack` ID from the CLI call), you can `ship` your `Formpack` to the network using the same CLI tool
```bash
form pack ship .
```

This process may take up to 5 minutes, you can `poll` the network for the status of your deployment with the `form manage` CLI tool. 

```bash
form manage get status <formpack-id>
```

This will return the state of the deployment (`PENDING`, `SUCCESS`, `FAILURE { reason: <some error message }`)

If your deployment is successful (which it should be, with very few exceptions).

If you run into an issue, please report it via github issues or join our discord where you can engage the core team.

<hr>

## Accessing your Instance

<hr>

## Committing Changes

<hr>

## Roadmap

<hr>
