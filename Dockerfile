FROM python:3.12-slim

# Prevent Python from writing out pyc files
ENV PYTHONDONTWRITEBYTECODE=1

# Prevent Python from buffering stdin/stdout
ENV PYTHONUNBUFFERED=1

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y build-essential python3-dev libldap2-dev libsasl2-dev && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -m django
WORKDIR /home/django

RUN python -m pip install --upgrade pip

COPY requirements.txt .
RUN python -m pip install --no-cache-dir -r requirements.txt

COPY . .
RUN chmod +x docker-entrypoint.sh

USER django

CMD [ "./docker-entrypoint.sh" ]
