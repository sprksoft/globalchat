group "default" {
  targets = [ "smppgc" ]
}

target "smppgc" {
  platforms = [ "linux/arm/v7", "linux/amd64" ]
  args = {
    RELEASE = "true"
  }

  target = "prod"
  tags = ["smppserver_smppgc:beta"]
}
