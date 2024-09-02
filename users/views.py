from django.contrib.auth import authenticate, login, logout
from django.contrib.auth.decorators import login_required
from django.contrib.auth.models import User
from django.shortcuts import render, redirect
from .forms import LoginForm

@login_required
def user_list(request):
    users = User.objects.all()
    return render(request, "users/list.html", {"users": users})

def login_view(request):
    if request.method == "POST":
        form = LoginForm(request.POST)

        if form.is_valid():
            username = form.cleaned_data["username"]
            password = form.cleaned_data["password"]
            user = authenticate(request, username=username, password=password)

            if user is not None:
                login(request, user)
                return redirect(request.GET.get("next", "/"))
            else:
                form.add_error(None, "Invalid credentials")
    else:
        form = LoginForm()

    return render(request, "users/login.html", {"form": form})

def logout_view(request):
    logout(request)
    return redirect("users:login")
