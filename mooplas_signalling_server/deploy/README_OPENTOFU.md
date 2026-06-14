# Deployment of a Mooplas signalling server

AWS infrastructure is managed with OpenTofu in [`opentofu/`](./opentofu/). Use that workflow instead of creating EC2, security groups, Elastic IPs, or CloudFront distributions in the AWS console.

TLS is terminated by CloudFront, so the container runs in plain `ws://` mode on the EC2 instance.

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

You can use an existing SSH public key instead, for example `~/.ssh/id_ed25519.pub`, as long as you use the matching private key for SSH.

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

OpenTofu does not build or publish the Docker image.

## 3 Get connection details

```bash
cd mooplas_signalling_server/deploy/opentofu
tofu output elastic_ip
tofu output cloudfront_domain_name
tofu output -raw signalling_server_url
tofu output ssh_command
```

Use `ec2-user` for the Amazon Linux instance.

## 4 Build the image on the local machine

Run these commands on the local machine, from the repository root:

```bash
docker build -f mooplas_signalling_server/Dockerfile -t mooplas-signalling-server:latest .
```

This compiles the Rust binary inside a container and produces a small runtime image.

## 5 Transfer the image to EC2

From the repository root:

```bash
docker save mooplas-signalling-server:latest | gzip > mooplas-signalling-server.tar.gz
scp -i ~/.ssh/mooplas-signalling mooplas-signalling-server.tar.gz ec2-user@<EC2_ELASTIC_IP>:/opt/mooplas-signalling/
```

You can get `<EC2_ELASTIC_IP>` with:

```bash
cd mooplas_signalling_server/deploy/opentofu
tofu output -raw elastic_ip
```

## 6 Load the image on EC2

SSH into the instance and load the image:

```bash
ssh -i ~/.ssh/mooplas-signalling ec2-user@<EC2_ELASTIC_IP> -v -o IdentitiesOnly=yes
cd /opt/mooplas-signalling
docker load < mooplas-signalling-server.tar.gz
```

Verify the image is available:

```bash
docker images mooplas-signalling-server
```

## 7 Start the server

OpenTofu writes `/opt/mooplas-signalling/docker-compose.yml` and `/opt/mooplas-signalling/.env` during EC2 bootstrap.

On the EC2 instance:

```bash
cd /opt/mooplas-signalling
docker compose up -d
```

Check it is running:

```bash
docker compose ps
docker compose logs
curl http://localhost:3536/health
```

## 8 Verify end-to-end

From the local machine:

```bash
# Direct to EC2, plain HTTP
curl http://<EC2_ELASTIC_IP>:3536/health

# Via CloudFront, TLS-terminated
curl https://<CLOUDFRONT_DOMAIN>/health
```

## 9 Build the game with the signalling server URL

Use the OpenTofu output as the signalling server URL in the game build:

```bash
cd mooplas_signalling_server/deploy/opentofu
SIGNALLING_SERVER_URL=$(tofu output -raw signalling_server_url)
```

Example:

```text
SIGNALLING_SERVER_URL=wss://d111111abcdef8.cloudfront.net
```

See [README.md](./../../README.md#how-to-build-wasm-for-the-web) at the repository root for game build instructions.

## 10 Updating the server

When you want to deploy a new version, repeat the image build and transfer steps, then restart:

```bash
docker build -f mooplas_signalling_server/Dockerfile -t mooplas-signalling-server:latest .
docker save mooplas-signalling-server:latest | gzip > mooplas-signalling-server.tar.gz
scp -i ~/.ssh/mooplas-signalling mooplas-signalling-server.tar.gz ec2-user@<EC2_ELASTIC_IP>:/opt/mooplas-signalling/
ssh -i ~/.ssh/mooplas-signalling ec2-user@<EC2_ELASTIC_IP> -v -o IdentitiesOnly=yes
cd /opt/mooplas-signalling
docker load < mooplas-signalling-server.tar.gz
docker compose up -d
```

## 11 Destroying AWS infrastructure

When you no longer need the server:

```bash
cd mooplas_signalling_server/deploy/opentofu
tofu destroy
```

CloudFront deletion can take several minutes while AWS disables and propagates the distribution deletion.
