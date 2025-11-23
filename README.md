<!-- exclude:start -->
# Litterbox

<p align="center">
  <img src="litterbox/assets/cat.svg" alt="Centered SVG" width="200"/>
</p>

[![Build and Test](https://github.com/Gerharddc/litterbox/actions/workflows/build-and-test.yml/badge.svg)](https://github.com/Gerharddc/litterbox/actions/workflows/build-and-test.yml)
<!-- exclude:end -->

Litterbox is a Linux sandbox environment catered to the needs of developers. Its primary goal is to provide SOME isolation between a containerised development environment and a host system. Its secondary goal is to provide a repeatable and documented environment for development.

The isolation goal is to prevent rogue processes (which might arrive through a supply chain attack or similar) from gaining access to sensitive files on your home directory or access to all of your SSH keys. Litterbox achieves file system isolation by restricting a development container to only have access to a single folder on your host system (and nothing above it). SSH key protection is achieved with a custom SSH agent that only exposes a limited number of SSH keys to a single Litterbox and prompts the user (via a pop-up window) before completing any signing requests.

N.B. Litterbox is free software that does not come with any warranty or guarantees. It is not an anti-malware solution and does not guarantee that your system will be protected from any attacks. Its goal is just to be BETTER THAN NOTHING but even that is not guaranteed. By using this software you agree that you are doing so at your own risk and take full responsibility for anything that might happen.

---

## Isolation limitations

The isolation/sandboxing provided by Litterbox is limited and still leaves open many holes and/or vulnerabilities. It is not intented to shield you from software that is known to be malicious so please do not run malware or untrusted software inside it deliberatly. Its only goal is to try and provide SOME damange limitation in the event that 3rd party software and/or code that you trust were to unexpectedly get compromised.

By design, Litterbox comes with AT LEAST the following limitation/vulnerabilities:

- Everything running inside a Litterbox is running on top of your host kernel in the same way as normal applications. Thus, anything running inside the Litterbox could still exploit vulnerabilities in your host kernel to gain full access to your system.
- Everything running inside a Litterbox has full access to your Wayland server in the same way as normal applications. Thus, anything running inside the Litterbox could still exploit vulnerabilities in your Wayland server to gain full access to your system.
- Since applications running inside a Litterbox have normal access to your Wayland server, they have full access to things such as your clipboard so you should avoid copying any sensitive data around while you have a Litterbox running.
- Litterbox relies on Podman as its container runtime. Thus, anything running inside a Litterbox could still exploit vulnerabilities in your Podman engine to gain full access to your system.
- Litterbox does not provide ANY network isolation. Anything running inside a Litterbox has fully access to your host's network (including localhost) in the same way a normal application running on your system would. You should therefore be very careful to not have anything sensitive and/or vulnerable accessible on your network.
- Litterbox hosts an SSH agent server powered by https://crates.io/crates/russh. The goal of this server is to provide restricted access to SSH keys inside a Litterbox through a shared socket. Thus, anything running inside a Litterbox could still exploit vulnerabilities in this library to gain full access to your system.

N.B. it is again emphasised that Litterbox does not come with any warranties or guarantees. Using it is at your own risk and the Litterbox authors do not accept any libiality for damages that might be incurred.

---

## Installation

Simply run the following commands:

```
curl -LO https://github.com/Gerharddc/litterbox/releases/latest/download/install.sh
sudo chmod +x install.sh
./install.sh
```

---

## Usage

TODO: write section

---

## Comparison to alternatives

### Full Virtual Machine

Even though good isolation can be achieved using a virtual machine, the idea with Litterbox is to provide decent isolation coupled with more convenience and less overhead. Litterbox runs everything on top of your host Linux kernel (thereby reducing overhead) and inside a folder that exists directly on the host (thereby making it simpler to share files). Furthermore, Litterbox allows applications to connect directly to the Wayland server on your host system which means that applications running inside the Litterbox are graphically composed just like normal applications and seamlessly have access to things like your clipboard (so be careful what you put in there).

N.B. copy and pasting files to/from the Litterbox currently won't work as expected in many cases since file paths inside and outside the Litterbox are different. Copying data rather than paths should work as normal though.

### DevContainers

Litterbox is very similar to DevContainers in that is uses Dockerfiles and containers to create a repeatable and somewhat isolated environment for a development project. A drawback with DevContainers though is that they are intended to be driven by an IDE and therefore require deep IDE integration to work properly. In practice this means that they are only really useable through VSCode (and maybe a few others). Litterbox tries to take a much more flexible approach in that it encourages you to instead run your entire IDE inside the container together with you project(s). This has the advantage that your IDE needs no knowledge of Litterbox and that your host system is also isolated from the IDE and any extensions that might be running inside it. Furthermore, you can easily develop multiple projects together inside a single Litterbox if you like since there isn't the same strong connection between a single code repository and a single DevContainer.

### Distrobox

Litterbox is most similar to Distrobox in terms of its design and functionality. The primary difference is that Distrobox does not aim to provide any isolation/sandboxing at all whereas Litterbox has a strong emphasis on providing it. Distrobox avoids sandboxing in order to provide more seamless integration between applications running inside the Distrobox and the host system. It tries to solve the problem of running software intended for a different distro as if it is running natively. Litterbox instead sacrificies much of the convenience that Distrobox provides in exchange for some isolation/sandboxing capabilities.

---

## TODO

Litterbox is still very much WIP with many missing features or required improvements. Following is a list of some important pieces that are still missing:

- [ ] Use `udica` to improve isolation on SELinux environments.
- [ ] Add automated testing.
- [ ] Expose limited DBus access to allow applications to open URLs.
- [ ] Make it possible to Xorg apps to running via Wayback integration.
- [ ] Add Dockerfile templates for more distros.
- [ ] Add optional support for network isolation.

---

## Contributing

Litterbox already meets most of my own needs and I have higher priority projects that I currently want to focus on instead. Hence, I will unfortunately not be able to spend much time (if any) on feature requests and bug reports. However, I would be more than happy to accept help in the form of PRs. Also please feel free to help out in any other way you see suitable!
