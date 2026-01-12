# Overview

`unifi-exporter` can be used to monitor Unifi API endpoints and will report metrics back
in a `node_exporter` style output for consumption by Prometheus. Data is not stored by
`unifi-exporter`, so no persistent storage is required. The intention is that all data
will instead be stored by Prometheus.

## Deployment
To deploy `unifi-exporter`, there are Kubernetes manifests located in the `manifests` directory.
These manifests can be modified to suite your requirements.

Fetch your `unifi` API token: https://assets.identity.ui.com/unifi-access/api_reference.pdf

The token needs to be `base64` encoded and added to the `unifi-secret.yaml` file:
```bash
❯ echo -n "example-token" | base64
ZXhhbXBsZS10b2tlbg==
```

Add the resulting output to the `unifi-secret.yaml` as the `UNIFI_API_KEY`:
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: unifi-exporter
  namespace: monitoring
type: Opaque
data:
  UNIFI_API_TOKEN: ZXhhbXBsZS10b2tlbg==
```

There are two additional environment variables that are used by the application, they are currently
defined in the `unifi-exporter-deployment.yaml` file:
```yaml
❯ yq '.spec.template.spec.containers[].env' unifi-exporter-deployment.yaml
- name: UNIFI_API_ENDPOINT
  value: "https://172.20.0.254"
- name: RUST_LOG
  value: info
- name: UNIFI_API_TOKEN
  valueFrom:
    secretKeyRef:
      name: unifi-exporter
      key: UNIFI_API_TOKEN
```

We can see that `UNIFI_API_TOKEN` will come from the `unifi-secret.yaml` manifest. But the other two
should be modified to suite your environment and requirements. Where `UNIFI_API_ENDPOINT` is the IP
or hostname of your Unifi controller.

Once the manifests have been modified as per your requirements, you can deploy the application with
`kubectl`:
```bash
cd manifests
kubectl apply -f .
```

![Alt text](./prometheus-screenshot.png?raw=true "Prometheus RX Screenshot")

