import time
import threading

def debounce(wait):
    def decorator(fn):
        last_call = [0]  # Using a list to make it mutable inside inner function
        def debounced(*args, **kwargs):
            def call_it():
                fn(*args, **kwargs)
            last_call[0] = time.time()
            threading.Timer(wait, lambda: call_it() if time.time() - last_call[0] >= wait else None).start()
        return debounced
    return decorator

# Example usage
@debounce(1.0)  # Debounce for 1 second
def my_function(arg):
    print(f'Function called with argument: {arg}')

# Simulating rapid calls
my_function('first')
my_function('second')
my_function('third')

# Sleep to ensure the program doesn't exit before the debounced function is called
time.sleep(2)
