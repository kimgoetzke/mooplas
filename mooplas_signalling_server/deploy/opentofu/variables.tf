variable "aws_region" {
  description = "AWS region for regional resources."
  type        = string
  default     = "eu-west-1"

  validation {
    condition     = can(regex("^[a-z]{2}-[a-z]+-[0-9]+$", var.aws_region))
    error_message = "aws_region must look like eu-west-1."
  }
}

variable "environment" {
  description = "Deployment environment tag/name suffix."
  type        = string
  default     = "prod"

  validation {
    condition     = can(regex("^[a-z0-9-]+$", var.environment))
    error_message = "environment must contain only lowercase letters, digits, and hyphens."
  }
}

variable "name_prefix" {
  description = "Prefix used for AWS resource names."
  type        = string
  default     = "mooplas-signalling"

  validation {
    condition     = can(regex("^[a-z0-9-]+$", var.name_prefix))
    error_message = "name_prefix must contain only lowercase letters, digits, and hyphens."
  }
}

variable "instance_type" {
  description = "EC2 instance type for the signalling server."
  type        = string
  default     = "t3.micro"

  validation {
    condition     = length(var.instance_type) > 0
    error_message = "instance_type must not be empty."
  }
}

variable "ssh_allowed_cidr" {
  description = "IPv4 CIDR allowed to SSH to the instance. Use your public IP with /32."
  type        = string

  validation {
    condition     = can(cidrhost(var.ssh_allowed_cidr, 0))
    error_message = "ssh_allowed_cidr must be a valid CIDR block, for example 203.0.113.10/32."
  }
}

variable "ssh_public_key" {
  description = "SSH public key content. Prefer this for CI or pass ssh_public_key_path for local use."
  type        = string
  default     = null
  nullable    = true

  validation {
    condition = var.ssh_public_key == null ? true : can(regex(
      "^(ssh-rsa|ssh-ed25519|ecdsa-sha2-nistp[0-9]+) ",
      trimspace(var.ssh_public_key),
    ))
    error_message = "ssh_public_key must be valid OpenSSH public key content when set."
  }
}

variable "ssh_public_key_path" {
  description = "Path to a local SSH public key file. Used only when ssh_public_key is unset."
  type        = string
  default     = null
  nullable    = true
}

variable "signalling_port" {
  description = "Port exposed by the signalling server and CloudFront origin."
  type        = number
  default     = 3536

  validation {
    condition     = var.signalling_port >= 1 && var.signalling_port <= 65535
    error_message = "signalling_port must be between 1 and 65535."
  }
}

variable "signalling_image" {
  description = "Docker image reference expected to be loaded manually on the instance. OpenTofu does not build or publish it."
  type        = string
  default     = "mooplas-signalling-server:latest"

  validation {
    condition     = length(var.signalling_image) > 0
    error_message = "signalling_image must not be empty."
  }
}

variable "vpc_cidr" {
  description = "CIDR block for the dedicated VPC."
  type        = string
  default     = "10.42.0.0/16"

  validation {
    condition     = can(cidrhost(var.vpc_cidr, 0))
    error_message = "vpc_cidr must be a valid CIDR block."
  }
}

variable "public_subnet_cidr" {
  description = "CIDR block for the public subnet."
  type        = string
  default     = "10.42.1.0/24"

  validation {
    condition     = can(cidrhost(var.public_subnet_cidr, 0))
    error_message = "public_subnet_cidr must be a valid CIDR block."
  }
}

variable "ami_id" {
  description = "Optional explicit AMI ID. When null, the latest Amazon Linux 2023 x86_64 AMI is selected."
  type        = string
  default     = null
  nullable    = true

  validation {
    condition     = var.ami_id == null ? true : can(regex("^ami-[a-f0-9]+$", var.ami_id))
    error_message = "ami_id must be a valid AMI ID when set."
  }
}

variable "ami_name_filter" {
  description = "AMI name filter used when ami_id is null."
  type        = string
  default     = "al2023-ami-2023.*-x86_64"
}

variable "ami_owner" {
  description = "AMI owner used when ami_id is null. Defaults to Amazon."
  type        = string
  default     = "amazon"
}

variable "cloudfront_price_class" {
  description = "CloudFront price class."
  type        = string
  default     = "PriceClass_100"

  validation {
    condition = contains([
      "PriceClass_100",
      "PriceClass_200",
      "PriceClass_All",
    ], var.cloudfront_price_class)
    error_message = "cloudfront_price_class must be PriceClass_100, PriceClass_200, or PriceClass_All."
  }
}

variable "cloudfront_ipv6_enabled" {
  description = "Whether CloudFront should serve IPv6 viewer traffic."
  type        = bool
  default     = true
}
