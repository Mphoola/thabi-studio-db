# Minimal SSH bastion for tunnel tests: password and key login, TCP
# forwarding on. The host key is generated when the image is built.
FROM alpine:3.20
RUN apk add --no-cache openssh \
 && ssh-keygen -A \
 && adduser -D thabi \
 && sed -i \
      -e 's/^#\?AllowTcpForwarding.*/AllowTcpForwarding yes/' \
      -e 's/^#\?PasswordAuthentication.*/PasswordAuthentication yes/' \
      /etc/ssh/sshd_config
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
EXPOSE 22
ENTRYPOINT ["/entrypoint.sh"]
