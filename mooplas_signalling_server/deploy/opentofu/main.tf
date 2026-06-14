data "aws_ami" "amazon_linux" {
  count = var.ami_id == null ? 1 : 0

  most_recent = true
  owners      = [var.ami_owner]

  filter {
    name   = "name"
    values = [var.ami_name_filter]
  }

  filter {
    name   = "architecture"
    values = ["x86_64"]
  }

  filter {
    name   = "virtualization-type"
    values = ["hvm"]
  }
}

resource "aws_vpc" "signalling" {
  cidr_block           = var.vpc_cidr
  enable_dns_hostnames = true
  enable_dns_support   = true

  tags = {
    Name = "${local.resource_name}-vpc"
  }
}

resource "aws_subnet" "public" {
  vpc_id                  = aws_vpc.signalling.id
  cidr_block              = var.public_subnet_cidr
  map_public_ip_on_launch = true

  tags = {
    Name = "${local.resource_name}-public-subnet"
  }
}

resource "aws_internet_gateway" "signalling" {
  vpc_id = aws_vpc.signalling.id

  tags = {
    Name = "${local.resource_name}-igw"
  }
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.signalling.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.signalling.id
  }

  tags = {
    Name = "${local.resource_name}-public-rt"
  }
}

resource "aws_route_table_association" "public" {
  subnet_id      = aws_subnet.public.id
  route_table_id = aws_route_table.public.id
}

resource "aws_key_pair" "signalling" {
  key_name   = local.resource_name
  public_key = local.ssh_public_key

  tags = {
    Name = "${local.resource_name}-key"
  }

  lifecycle {
    precondition {
      condition     = length(local.ssh_public_key) > 0
      error_message = "Set either ssh_public_key or ssh_public_key_path. Do not use a private key here."
    }
  }
}

resource "aws_security_group" "signalling" {
  name        = "${local.resource_name}-sg"
  description = "Mooplas signalling server ingress and bootstrap egress"
  vpc_id      = aws_vpc.signalling.id

  tags = {
    Name = "${local.resource_name}-sg"
  }
}

resource "aws_vpc_security_group_ingress_rule" "ssh" {
  security_group_id = aws_security_group.signalling.id
  description       = "SSH from configured administrator CIDR"
  from_port         = 22
  to_port           = 22
  ip_protocol       = "tcp"
  cidr_ipv4         = var.ssh_allowed_cidr

  tags = {
    Name = "${local.resource_name}-ssh-ingress"
  }
}

resource "aws_vpc_security_group_ingress_rule" "signalling" {
  security_group_id = aws_security_group.signalling.id
  description       = "Signalling server from anywhere IPv4; CloudFront origin and direct health checks"
  from_port         = var.signalling_port
  to_port           = var.signalling_port
  ip_protocol       = "tcp"
  cidr_ipv4         = "0.0.0.0/0"

  tags = {
    Name = "${local.resource_name}-signalling-ingress"
  }
}

resource "aws_vpc_security_group_egress_rule" "https" {
  security_group_id = aws_security_group.signalling.id
  description       = "Outbound HTTPS for package updates, Docker, and Compose bootstrap"
  from_port         = 443
  to_port           = 443
  ip_protocol       = "tcp"
  cidr_ipv4         = "0.0.0.0/0"

  tags = {
    Name = "${local.resource_name}-https-egress"
  }
}

resource "aws_instance" "signalling" {
  ami                         = local.selected_ami_id
  instance_type               = var.instance_type
  subnet_id                   = aws_subnet.public.id
  vpc_security_group_ids      = [aws_security_group.signalling.id]
  key_name                    = aws_key_pair.signalling.key_name
  associate_public_ip_address = true
  user_data                   = local.user_data

  root_block_device {
    volume_type = "standard"
    volume_size = 8
    encrypted   = true

    tags = {
      Name = "${local.resource_name}-root"
    }
  }

  tags = {
    Name = local.resource_name
  }
}

resource "aws_eip" "signalling" {
  domain = "vpc"

  tags = {
    Name = "${local.resource_name}-eip"
  }
}

resource "aws_eip_association" "signalling" {
  instance_id   = aws_instance.signalling.id
  allocation_id = aws_eip.signalling.allocation_id
}

data "aws_cloudfront_cache_policy" "caching_disabled" {
  name = "Managed-CachingDisabled"
}

data "aws_cloudfront_origin_request_policy" "all_viewer" {
  name = "Managed-AllViewer"
}

resource "aws_cloudfront_distribution" "signalling" {
  enabled         = true
  is_ipv6_enabled = var.cloudfront_ipv6_enabled
  comment         = "Mooplas signalling server ${var.environment}"
  price_class     = var.cloudfront_price_class

  origin {
    domain_name = aws_eip.signalling.public_dns
    origin_id   = "${local.resource_name}-ec2-origin"

    custom_origin_config {
      http_port              = var.signalling_port
      https_port             = 443
      origin_protocol_policy = "http-only"
      origin_ssl_protocols   = ["TLSv1.2"]
    }
  }

  default_cache_behavior {
    target_origin_id         = "${local.resource_name}-ec2-origin"
    viewer_protocol_policy   = "https-only"
    cache_policy_id          = data.aws_cloudfront_cache_policy.caching_disabled.id
    origin_request_policy_id = data.aws_cloudfront_origin_request_policy.all_viewer.id

    allowed_methods = [
      "GET",
      "HEAD",
      "OPTIONS",
      "PUT",
      "POST",
      "PATCH",
      "DELETE",
    ]

    cached_methods = [
      "GET",
      "HEAD",
    ]
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  viewer_certificate {
    cloudfront_default_certificate = true
  }

  tags = {
    Name = "${local.resource_name}-cloudfront"
  }

  depends_on = [aws_eip_association.signalling]
}
