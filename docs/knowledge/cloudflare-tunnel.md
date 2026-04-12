# Cloudflare Tunnel Setup

Cloudflare Tunnel (formerly Argo Tunnel) exposes internal services to the internet through Cloudflare's edge, without needing public IPs or firewall rules.

## Reference: ui-proxy Tunnel

The canonical example is in `~/projects/dragb/infra/k8s/cloudflared/`.

### 1. Tunnel Credentials

Cloudflare tunnels require a credentials file with your tunnel ID and account token. This is stored as a Kubernetes Secret:

```yaml
# kubectl create secret generic cloudflared-credentials \
#   --from-file=credentials.json=<(cat << 'JSON'
# {"AccountTag":"...","TunnelID":"...","TunnelName":"..."}
# JSON
# )
```

### 2. ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: cloudflared-config
  namespace: dragb
data:
  config.yml: |
    tunnel: <tunnel-id>          # e.g. 43b53dec-70ea-40b7-a0ec-d16fb80f8b14
    credentials-file: /etc/cloudflared/credentials.json
    ingress:
      - hostname: uiproxy.yuacx.com
        service: http://ui-proxy.dragb.svc.cluster.local:26341
      - hostname: hebei.yuacx.com
        service: http://hebei-analysis.dragb.svc.cluster.local:8090
      - service: http_status:404   # catch-all
```

### 3. Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: cloudflared
  namespace: dragb
spec:
  replicas: 1
  selector:
    matchLabels:
      app: cloudflared
  template:
    spec:
      containers:
        - name: cloudflared
          image: cloudflare/cloudflared:latest
          args:
            - tunnel
            - --config
            - /etc/cloudflared/config.yml
            - run
          volumeMounts:
            - name: config
              mountPath: /etc/cloudflared/config.yml
              subPath: config.yml
              readOnly: true
            - name: credentials
              mountPath: /etc/cloudflared/credentials.json
              subPath: credentials.json
              readOnly: true
      volumes:
        - name: config
          configMap:
            name: cloudflared-config
        - name: credentials
          secret:
            secretName: cloudflared-credentials
```

### 4. Add a New Service

To expose a new service (e.g. `browser.yuacx.com` for Chrome CDP):

1. Edit the `config.yml` in the ConfigMap and add an ingress rule **before** the catch-all:

```yaml
ingress:
  - hostname: browser.yuacx.com
    service: http://browser-yuacx:9222
  - hostname: uiproxy.yuacx.com
    service: http://ui-proxy.dragb.svc.cluster.local:26341
  - service: http_status:404
```

2. Create a DNS CNAME record in Cloudflare dashboard:
   - Type: CNAME
   - Name: `browser`
   - Target: `<tunnel-id>.cfargotunnel.com`
   - Proxy status: DNS only (Cloudflare handles proxy automatically for tunneled routes)

3. Roll the cloudflared deployment to pick up the new config:
```bash
kubectl rollout restart deployment/cloudflared -n dragb
```

### 5. Create a New Tunnel

If you need a separate tunnel (e.g. for a different domain or isolation):

1. Create the tunnel:
```bash
cloudflared tunnel create <name>
# Outputs: Tunnel ID and credentials file path
```

2. Create the credentials Secret in k8s:
```bash
kubectl create secret generic cloudflared-credentials-new \
  --from-file=credentials.json=/path/to/credentials.json \
  -n dragb
```

3. Update the ConfigMap with the new tunnel ID and credentials secret name

4. Add DNS records pointing to the new tunnel:
```bash
cloudflared tunnel route dns <name> browser.yuacx.com
```

### 6. Common Issues

- **404 on all routes**: Check the catch-all is last in ingress rules
- **502 errors**: Verify the backend service is running and reachable from within the cluster
- **Tunnel not connecting**: Check cloudflared logs `kubectl logs -n dragb deployment/cloudflared`
- **DNS not propagating**: Set proxy status to "DNS only" first, switch to "Proxied" after tunnel is verified

## Key Files

- `~/projects/dragb/infra/k8s/cloudflared/deployment.yaml`
- `~/projects/dragb/infra/k8s/cloudflared/configmap.yaml`
