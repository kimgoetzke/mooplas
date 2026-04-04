# Manual EC2 deployment of a Mooplas signalling server

This guide is just a reference for deploying the signalling server to an EC2 instance manually (no CI/CD). It's the
most basic, low-cost, but manual way of setting up a server I could think of. All the following is done in the AWS
console.

TLS is terminated by CloudFront, so the container runs in plain `ws://` mode.

## 1 Create the EC2 instance

### 1A. Create an SSH key pair

1. Go to **EC2 > Key Pairs** (under Network & Security in the left sidebar)
2. Click **Create key pair**
3. Enter a name
4. Select aa as the key pair type and a file format
5. Click **Create key pair** - the `.pem` file downloads automatically
6. Move it somewhere safe and restrict permissions:

   ```bash
   mv ~/Downloads/mooplas-signalling.pem ~/.ssh/
   chmod 400 ~/.ssh/mooplas-signalling.pem
   ```

### 1B. Create a security group

1. Go to **EC2 > Security Groups** (under Network & Security in the left sidebar)
2. Click **Create security group**
3. Enter a name, etc.
4. Under **Inbound rules**, add two rules:
    - **Type:** SSH, **Port:** 22, **Source:** My IP (this restricts SSH access to the current IP address)
    - **Type:** Custom TCP, **Port:** 3536, **Source:** Anywhere-IPv4 (`0.0.0.0/0`) (CloudFront connects from
      various IPs)
5. Under **Outbound rules**, add one rule:
    - **Type:** HTTPS, **Port:** 443, **Destination:** 0.0.0.0/0 (allows the server to make outbound HTTPS requests,
      e.g. for updates)
6. Click **Create security group**

### 1C. Launch the EC2 instance

1. Go to **EC2 > Instances** and click **Launch instances**
2. Enter a name
3. Select **Amazon Linux** as the AMI (Amazon Machine Image)
4. Select whatever machine type you want
5. Under **Key pair**, select the key pair you created in step 1A
6. Under **Network settings**, click **Select existing security group** and choose the one from step 1B
7. Click **Launch instance**

### 1D. Create and assign an Elastic IP

An Elastic IP is a static public IP address. Without one, the instance gets a new IP every time it stops and starts,
which breaks the CloudFront origin.

1. Go to **EC2 > Elastic IPs**
2. Click **Allocate Elastic IP address**, then **Allocate**
3. Select the new Elastic IP, click **Actions > Associate Elastic IP address**
4. Select your instance from the dropdown and click **Associate**

Note the Elastic IP - this is what is used as `<EC2_ELASTIC_IP>` throughout this guide.

## 2 SSH into the instance

```bash
chmod 400 ~/path/to/key.pem
ssh -i ~/path/to/key.pem <EC2_USER>@<EC2_ELASTIC_IP> -v -o IdentitiesOnly=yes
```

`<EC2_USER>` is `ubuntu` on Ubuntu images or `ec2-user` on Amazon Linux.

## 3 Install Docker on the instance

Run these commands on the EC2 instance.

Ubuntu:

```bash
sudo dnf update -y
sudo dnf install -y docker
sudo systemctl enable docker
sudo systemctl start docker
sudo usermod -aG docker $USER
```

For docker-compose, it's not bundled separately on Amazon Linux - install it as a plugin:

```bash
sudo mkdir -p /usr/local/lib/docker/cli-plugins
sudo curl -SL https://github.com/docker/compose/releases/latest/download/docker-compose-linux-x86_64 -o /usr/local/lib/docker/cli-plugins/docker-compose
sudo chmod +x /usr/local/lib/docker/cli-plugins/docker-compose
```

Log out and back in for the group change to take effect:

```bash
exit
ssh -i ~/path/to/key.pem <EC2_USER>@<EC2_ELASTIC_IP> -v -o IdentitiesOnly=yes
```

Verify Docker is working:

```bash
docker --version
docker compose version
```

## 4 Create the deployment directory on the instance

Run these commands on the EC2 instance:

```bash
sudo mkdir -p /opt/mooplas-signalling
sudo chown $USER:$USER /opt/mooplas-signalling
```

## 5 Build the image on the local machine

Run these commands on the local machine, from the repository root:

```bash
docker build -f mooplas_signalling_server/Dockerfile -t mooplas-signalling-server:latest .
```

This compiles the Rust binary inside a container and produces a small runtime image.

## 6 Transfer the image to EC2

Still on your development machine:

```bash
docker save mooplas-signalling-server:latest | gzip > mooplas-signalling-server.tar.gz
```

Copy the compressed image to the EC2 instance:

```bash
scp -i ~/path/to/key.pem mooplas-signalling-server.tar.gz <EC2_USER>@<EC2_ELASTIC_IP>:/opt/mooplas-signalling/
```

## 7 Load the image on EC2

SSH into the instance and load the image:

```bash
ssh -i ~/path/to/key.pem <EC2_USER>@<EC2_ELASTIC_IP> -v -o IdentitiesOnly=yes
cd /opt/mooplas-signalling
docker load < mooplas-signalling-server.tar.gz
```

Verify the image is available:

```bash
docker images mooplas-signalling-server
```

## 8 Copy the Compose file and create the environment file

On the local machine, copy the Compose file to EC2:

```bash
scp -i ~/path/to/key.pem mooplas_signalling_server/deploy/docker-compose.yml <EC2_USER>@<EC2_ELASTIC_IP>:/opt/mooplas-signalling/
```

Then on the EC2 instance, create the `.env` file:

```bash
cat > /opt/mooplas-signalling/.env << 'EOF'
SIGNALLING_IMAGE=mooplas-signalling-server:latest
HOST_PORT=3536
SIGNALLING_PORT=3536
EOF
```

## 9 Start the server

On the EC2 instance:

```bash
cd /opt/mooplas-signalling
docker compose up -d
```

Check it is running:

```bash
docker compose ps
```

Check the logs if necessary:

```bash
docker compose logs
```

Test the health endpoint from the EC2 instance:

```bash
curl http://localhost:3536/health
```

## 10 Create a CloudFront distribution

Back in the AWS console:

1. Go to **CloudFront**
2. Select **Free plan ($0/month)** somewhere to start the wizard
3. **Origin domain** - enter your EC2 Elastic IP or public DNS (e.g. `ec2-1-2-3-4.eu-west-1.compute.amazonaws.com`)
4. **Protocol** - set origin protocol policy to **HTTP only**
5. **HTTP port** - set to `3536`
6. **Cache policy** - select `CachingDisabled` (signalling traffic must not be cached)
7. **Origin request policy** - select `AllViewer` (forwards the WebSocket upgrade headers)
8. **Viewer protocol policy** - set to **HTTPS only** (clients connect via `wss://`)
9. **Allowed HTTP methods** - select `GET, HEAD, OPTIONS, PUT, POST, PATCH, DELETE`
10. Click **Create distribution**

Wait a few minutes until status changes from `Deploying` to `Enabled`.

Then test the domain:

```bash
curl https://<SOMETHING>.cloudfront.net/health
```

## 11 Verify end-to-end

From the local machine, both of these should work:

```bash
# Direct to EC2 (plain HTTP)
curl http://<EC2_ELASTIC_IP>:3536/health

# Via CloudFront (TLS-terminated)
curl https://<SOMETHING>.cloudfront.net/health
```

## 11 Build the game with the signalling server URL

Use this as the signalling server URL in the game build:

```
SIGNALLING_SERVER_URL=wss://<SOMETHING>.cloudfront.net
```

See [README.md](./../../README.md#how-to-build-wasm-for-the-web) at the root of the repository for instructions on how
to build the game with a signalling server URL configured.

## 13 Updating the server

When you want to deploy a new version, repeat steps 5-7 and restart:

1. **Build** on your development machine:

   ```bash
   docker build -f mooplas_signalling_server/Dockerfile -t mooplas-signalling-server:latest .
   ```

2. **Save and transfer**:

   ```bash
   docker save mooplas-signalling-server:latest | gzip > mooplas-signalling-server.tar.gz
   scp -i ~/path/to/key.pem mooplas-signalling-server.tar.gz <EC2_USER>@<EC2_ELASTIC_IP>:/opt/mooplas-signalling/
   ```

3. **Load and restart** on the EC2 instance:

   ```bash
   cd /opt/mooplas-signalling
   docker load < mooplas-signalling-server.tar.gz
   docker compose up -d
   ```
