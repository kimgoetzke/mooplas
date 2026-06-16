# Deployment of a Mooplas signalling server

AWS infrastructure is managed with OpenTofu in [`opentofu/`](./opentofu/). Use that workflow instead of creating EC2,
security groups, Elastic IPs, or CloudFront distributions in the AWS console.

TLS is terminated by CloudFront, so the container runs in plain `ws://` mode on the EC2 instance.

## Costs

This configuration creates resources that can incur AWS charges:

- EC2 instance
- Elastic IP/public IPv4 address
- CloudFront distribution and data transfer

The dedicated VPC, subnet, route table, and internet gateway do not materially add cost by themselves. No NAT Gateway is
created.

## 1 Create a local SSH key pair

Create the key pair locally, then give OpenTofu only the public key.

```bash
ssh-keygen -t ed25519 -f ~/.ssh/mooplas-signalling-server-ot -C "mooplas-signalling-server-ot"
chmod 600 ~/.ssh/mooplas-signalling-server-ot
```

This creates two files:

```text
~/.ssh/mooplas-signalling-server-ot      # private key; use with ssh and scp
~/.ssh/mooplas-signalling-server-ot.pub  # public key; give to OpenTofu
```

Set this in `mooplas_signalling_server/deploy/opentofu/mooplas.auto.tfvars`:

```hcl
ssh_public_key_path = "/home/you/.ssh/mooplas-signalling-server-ot.pub"
```

You can use an existing SSH public key instead, for example `~/.ssh/id_ed25519.pub`, as long as you use the matching
private key for SSH.

## 2 Create the AWS infrastructure

```bash
cd mooplas_signalling_server/deploy/opentofu
cp mooplas.auto.tfvars.example mooplas.auto.tfvars
# edit mooplas.auto.tfvars
tofu init
tofu plan
tofu apply
```

OpenTofu creates:

- dedicated public VPC networking with no NAT Gateway
- EC2 key pair registration from your public key
- security group with SSH from your CIDR, signalling port `3536` from anywhere IPv4, and HTTPS-only egress
- Amazon Linux EC2 instance with Docker and Compose bootstrapped
- Elastic IP
- CloudFront distribution for `wss://`
  ?
  OpenTofu does not build or publish the Docker image.

## 3 Export connection details

Run this after `tofu apply`. Run it again in any new terminal before using later commands.

```bash
cd ../../.. # Back to repo root
export MOOPLAS_SIGNALLING_SSH_KEY="$HOME/.ssh/mooplas-signalling-server-ot"
export MOOPLAS_SIGNALLING_OPENTOFU_DIR="$PWD/mooplas_signalling_server/deploy/opentofu"
export EC2_ELASTIC_IP="$(tofu -chdir="$MOOPLAS_SIGNALLING_OPENTOFU_DIR" output -raw elastic_ip)"
export CLOUDFRONT_DOMAIN="$(tofu -chdir="$MOOPLAS_SIGNALLING_OPENTOFU_DIR" output -raw cloudfront_domain_name)"
export SIGNALLING_SERVER_URL="$(tofu -chdir="$MOOPLAS_SIGNALLING_OPENTOFU_DIR" output -raw signalling_server_url)"
```

Check the values:

```bash
printf 'EC2_ELASTIC_IP=%s\nCLOUDFRONT_DOMAIN=%s\nSIGNALLING_SERVER_URL=%s\n' "$EC2_ELASTIC_IP" "$CLOUDFRONT_DOMAIN" "$SIGNALLING_SERVER_URL"
```

Use `ec2-user` for the Amazon Linux instance.

## 4 Build the image on the local machine

Run this from the repository root:

```bash
docker build -f mooplas_signalling_server/Dockerfile -t mooplas-signalling-server:latest .
```

This compiles the Rust binary inside a container and produces a small runtime image.

## 5 Transfer the image to EC2

Run this from the repository root:

```bash
docker save mooplas-signalling-server:latest | gzip > mooplas-signalling-server.tar.gz
scp -i "$MOOPLAS_SIGNALLING_SSH_KEY" mooplas-signalling-server.tar.gz "ec2-user@$EC2_ELASTIC_IP:/opt/mooplas-signalling/mooplas-signalling-server.tar.gz"
```

## 6 Load the image on EC2

Run this from the local machine:

```bash
ssh -i "$MOOPLAS_SIGNALLING_SSH_KEY" "ec2-user@$EC2_ELASTIC_IP" -v -o IdentitiesOnly=yes <<'EOF'
cd /opt/mooplas-signalling
docker load < mooplas-signalling-server.tar.gz
docker images mooplas-signalling-server
EOF
```

## 7 Start the server

OpenTofu writes `/opt/mooplas-signalling/docker-compose.yml` and `/opt/mooplas-signalling/.env` during EC2 bootstrap.

Run this from the local machine:

```bash
ssh -i "$MOOPLAS_SIGNALLING_SSH_KEY" "ec2-user@$EC2_ELASTIC_IP" -v -o IdentitiesOnly=yes <<'EOF'
cd /opt/mooplas-signalling
docker compose up -d
docker compose ps
docker compose logs
curl http://localhost:3536/health
EOF
```

## 8 Verify end-to-end

Run this from the local machine:

```bash
curl "http://$EC2_ELASTIC_IP:3536/health"
curl "https://$CLOUDFRONT_DOMAIN/health"
```

## 9 Build the game with the signalling server URL

Use the exported OpenTofu output as the signalling server URL in the game build:

```bash
printf 'SIGNALLING_SERVER_URL=%s\n' "$SIGNALLING_SERVER_URL"
```

Example value:

```text
SIGNALLING_SERVER_URL=wss://d111111abcdef8.cloudfront.net
```

See [README.md](./../../README.md#how-to-build-wasm-for-the-web) at the repository root for game build instructions.

## 10 Updating the server

When you want to deploy a new version, repeat the image build and transfer steps, then restart:

```bash
docker build -f mooplas_signalling_server/Dockerfile -t mooplas-signalling-server:latest .
docker save mooplas-signalling-server:latest | gzip > mooplas-signalling-server.tar.gz
scp -i "$MOOPLAS_SIGNALLING_SSH_KEY" mooplas-signalling-server.tar.gz "ec2-user@$EC2_ELASTIC_IP:/opt/mooplas-signalling/mooplas-signalling-server.tar.gz"
ssh -i "$MOOPLAS_SIGNALLING_SSH_KEY" "ec2-user@$EC2_ELASTIC_IP" -v -o IdentitiesOnly=yes <<'EOF'
cd /opt/mooplas-signalling
docker load < mooplas-signalling-server.tar.gz
docker compose up -d
EOF
```

## 11 Destroying AWS infrastructure

When you no longer need the server:

```bash
tofu -chdir="$MOOPLAS_SIGNALLING_OPENTOFU_DIR" destroy
```

CloudFront deletion can take several minutes while AWS disables and propagates the distribution deletion.
