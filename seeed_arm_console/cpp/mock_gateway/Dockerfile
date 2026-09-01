ARG ROS_IMAGE=swr.cn-north-4.myhuaweicloud.com/ddn-k8s/docker.io/osrf/ros:jazzy-desktop-full
FROM ${ROS_IMAGE}

ARG MUJOCO_VERSION=3.12.0
ARG MUJOCO_SHA256=a9367911e6d5eaeade17c2197304687421c1fc932cdf7bcd4cb8cfaf0374dcb2
ARG MUJOCO_URL=https://github.com/google-deepmind/mujoco/releases/download/3.12.0/mujoco-3.12.0-linux-x86_64.tar.gz
ENV MUJOCO_ROOT=/opt/mujoco
ENV LD_LIBRARY_PATH=/opt/mujoco/lib:${LD_LIBRARY_PATH}

RUN curl --fail --show-error --location \
        --connect-timeout 20 --max-time 180 \
        --retry 4 --retry-delay 3 --retry-all-errors \
        "${MUJOCO_URL}" \
        -o /tmp/mujoco.tar.gz \
    && echo "${MUJOCO_SHA256}  /tmp/mujoco.tar.gz" | sha256sum -c - \
    && mkdir -p /opt/mujoco \
    && tar -xzf /tmp/mujoco.tar.gz --strip-components=1 -C /opt/mujoco \
    && rm -f /tmp/mujoco.tar.gz
