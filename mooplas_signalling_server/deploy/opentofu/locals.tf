locals {
  resource_name = "${var.name_prefix}-${var.environment}"

  common_tags = {
    Project     = "mooplas"
    Application = "mooplas_signalling_server"
    Component   = "signalling"
    ManagedBy   = "opentofu"
    Environment = var.environment
  }

  ssh_public_key_from_path = var.ssh_public_key_path == null ? null : try(file(var.ssh_public_key_path), null)
  ssh_public_key           = trimspace(coalesce(var.ssh_public_key, local.ssh_public_key_from_path, ""))

  selected_ami_id = var.ami_id != null ? var.ami_id : data.aws_ami.amazon_linux[0].id

  deployment_directory = "/opt/mooplas-signalling"

  docker_compose_content = file("${path.module}/../docker-compose.yml")

  user_data = templatefile("${path.module}/user_data.sh.tftpl", {
    deployment_directory = local.deployment_directory
    docker_compose       = local.docker_compose_content
    signalling_image     = var.signalling_image
    signalling_port      = var.signalling_port
  })
}
