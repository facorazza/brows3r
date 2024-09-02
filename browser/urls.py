from django.urls import path, re_path

from . import views

app_name = "browser"

urlpatterns = [
    path("create-directory/", views.create_directory, name="create_directory"),
    re_path(r"^delete/(?P<path>.*)?$", views.delete, name="delete"),
    re_path("^download/(?P<path>.*)?$", views.download, name="download"),
    path("upload/", views.upload, name="upload"),
    re_path(r"^(?P<path>.*)?$", views.list, name="list"),
    # path("move/", views.move, name="move"),
]
