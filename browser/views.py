import logging

import boto3
import botocore.exceptions

from django.conf import settings
from django.contrib.auth.decorators import login_required
from django.http import HttpResponse
from django.shortcuts import render, redirect, get_object_or_404
from django.views.decorators.http import require_http_methods


logger = logging.getLogger(__name__)

s3 = boto3.client(
    "s3",
    endpoint_url=settings.S3_URL,
    aws_access_key_id=settings.S3_ACCESS_KEY_ID,
    aws_secret_access_key=settings.S3_ACCESS_KEY_SECRET,
    region_name=settings.S3_REGION,
)

@login_required
@require_http_methods(["GET"])
def list(request, path):
    prefix = f"{request.user.username}/"
    if path:
        prefix += f"{path}/"

    response = s3.list_objects_v2(
        Bucket=settings.S3_BUCKET,
        Prefix=prefix,
        Delimiter="/",
    )

    files = [
        {
            "full_path": f["Key"][len(request.user.username) + 1:],
            "name": f["Key"][len(prefix):],
            "size": f["Size"],
            "last_modified": f["LastModified"]
        }
        for f in response.get("Contents", []) if f["Key"] != prefix
    ]
    directories = [
        {
            "full_path": d["Prefix"][len(request.user.username) + 1:],
            "name": d["Prefix"][len(prefix):-1]
        }
        for d in response.get("CommonPrefixes", [])
    ]

    return render(
        request,
        "browser/list.html",
        {
            "path": path,
            "files": files,
            "directories": directories,
        }
    )

@login_required
@require_http_methods(["POST"])
def create_directory(request):
    path = request.POST.get("path")
    new_dir = request.POST.get("new_dir")

    s3.put_object(
        Bucket=settings.S3_BUCKET,
        Key="/".join(filter(None, [request.user.username, path, new_dir]))
    )
    return redirect("browser:list", path="/".join(filter(None, [path, new_dir])))

@login_required
@require_http_methods(["GET"])
def delete(request, path):
    full_path = f"{request.user.username}/{path}"

    is_directory = path.endswith("/")
    if is_directory:
        delete_directory_recursive(full_path)
        path = "/".join(path.split("/")[:-2])
    else:
        s3.delete_object(
            Bucket=settings.S3_BUCKET,
            Key=full_path,
        )
        path = "/".join(path.split("/")[:-1])
    return redirect("browser:list", path=path)

def delete_directory_recursive(prefix):
    paginator = s3.get_paginator("list_objects_v2")
    pages = paginator.paginate(
        Bucket=settings.S3_BUCKET,
        Prefix=prefix,
    )

    for page in pages:
        if "Contents" in page:
            objects_to_delete = [
                {"Key": obj["Key"]} for obj in page["Contents"]
            ]
            s3.delete_objects(
                Bucket=settings.S3_BUCKET,
                Delete={"Objects": objects_to_delete},
            )

@login_required
@require_http_methods(["GET"])
def download(request, path):
    full_path = f"{request.user.username}/{path}"

    try:
        file = s3.get_object(
            Bucket=settings.S3_BUCKET,
            Key=full_path,
        )
        response = HttpResponse(file["Body"].read())
        response["Content-Type"] = file["ContentType"]
        response["Content-Disposition"] = f"attachment; filename={path.split('/')[-1]}"
        return response
    except botocore.exceptions.ClientError:
        return HttpResponse("File not found", status=404)

@login_required
@require_http_methods(["POST"])
def upload(request):
    file = request.FILES["file"]
    path = request.POST.get("path", "")
    s3.upload_fileobj(
        file,
        settings.S3_BUCKET,
        f"{request.user.username}/{path}/{file.name}",
    )
    return redirect("browser:list", path=path)

# def move_item(request, source_path):
#     if request.method == "POST":
#         destination_path = request.POST["destination_path"]
#         is_directory = source_path.endswith("/")

#         if is_directory:
#             copy_directory_recursive(source_path, destination_path)
#             delete_directory_recursive(source_path)
#         else:
#             s3.copy_object(
# Bucket=settings.S3_BUCKET,
#                            CopySource={"
# Bucket": settings.S3_BUCKET, "Key": source_path},
#                            Key=destination_path)
#             s3.delete_object(
# Bucket=settings.S3_BUCKET, Key=source_path)

#         return redirect("list_files", path="/".join(destination_path.split("/")[:-1]) + "/")
#     return render(request, "move_item.html", {"source_path": source_path})

# def rename_item(request, old_path):
#     if request.method == "POST":
#         new_name = request.POST["new_name"]
#         is_directory = old_path.endswith("/")

#         if is_directory:
#             rename_directory_recursive(old_path, new_name)
#         else:
#             new_path = "/".join(old_path.split("/")[:-1] + [new_name])
#             s3.copy_object(
# Bucket=settings.S3_BUCKET,
#                            CopySource={"
# Bucket": settings.S3_BUCKET, "Key": old_path},
#                            Key=new_path)
#             s3.delete_object(
# Bucket=settings.S3_BUCKET, Key=old_path)

#         return redirect("list_files", path="/".join(new_path.split("/")[:-1]) + "/")
#     return render(request, "rename_item.html", {"old_path": old_path})

# def rename_directory_recursive(old_prefix, new_name):
#     new_prefix = "/".join(old_prefix.split("/")[:-2] + [new_name]) + "/"
#     paginator = s3.get_paginator("list_objects_v2")
#     for page in paginator.paginate(
# Bucket=settings.S3_BUCKET, Prefix=old_prefix):
#         for obj in page.get("Contents", []):
#             old_key = obj["Key"]
#             new_key = new_prefix + old_key[len(old_prefix):]
#             s3.copy_object(
# Bucket=settings.S3_BUCKET,
#                            CopySource={"
# Bucket": settings.S3_BUCKET, "Key": old_key},
#                            Key=new_key)
#             s3.delete_object(
# Bucket=settings.S3_BUCKET, Key=old_key)

# def copy_item(request, source_path):
#     if request.method == "POST":
#         destination_path = request.POST["destination_path"]
#         is_directory = source_path.endswith("/")

#         if is_directory:
#             copy_directory_recursive(source_path, destination_path)
#         else:
#             s3.copy_object(
# Bucket=settings.S3_BUCKET,
#                            CopySource={"
# Bucket": settings.S3_BUCKET, "Key": source_path},
#                            Key=destination_path)

#         return redirect("list_files", path="/".join(destination_path.split("/")[:-1]) + "/")
#     return render(request, "copy_item.html", {"source_path": source_path})

# def copy_directory_recursive(source_prefix, destination_prefix):
#     paginator = s3.get_paginator("list_objects_v2")
#     for page in paginator.paginate(
# Bucket=settings.S3_BUCKET, Prefix=source_prefix):
#         for obj in page.get("Contents", []):
#             old_key = obj["Key"]
#             new_key = destination_prefix + old_key[len(source_prefix):]
#             s3.copy_object(
# Bucket=settings.S3_BUCKET,
#                            CopySource={"
# Bucket": settings.S3_BUCKET, "Key": old_key},
#                            Key=new_key)

# def move_item(request, source_path):
#     if request.method == "POST":
#         destination_path = request.POST["destination_path"]
#         is_directory = source_path.endswith("/")

#         if is_directory:
#             copy_directory_recursive(source_path, destination_path)
#             delete_directory_recursive(source_path)
#         else:
#             s3.copy_object(
# Bucket=settings.S3_BUCKET,
#                            CopySource={"
# Bucket": settings.S3_BUCKET, "Key": source_path},
#                            Key=destination_path)
#             s3.delete_object(
# Bucket=settings.S3_BUCKET, Key=source_path)

#         return redirect("list_files", path="/".join(destination_path.split("/")[:-1]) + "/")
#     return render(request, "move_item.html", {"source_path": source_path})
