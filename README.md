# ProcessMonitor
Rust program that reads a configuration from yaml file and alerts when process are not available.

It uses a configuration file like the following:

```   
process:
  - name: "atom.exe"
    count: 1

  - name: "chrome.exe"
    count: 2
```

and the following environment variables:

- SLACK_HOOK
- SLACK_CHANNEL
- SLACK_USER

If the process count in .yml does not match the current processes, it sends a message to a Slack channel.
By default it runs every 2 minutes.

Tested in Windows 10.


