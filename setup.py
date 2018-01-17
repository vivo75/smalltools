from distutils.core import setup

from setuptools import setup, find_packages
from codecs import open
from os import path

here = path.abspath(path.dirname(__file__))

# Get the long description from the README file
with open(path.join(here, 'README.md'), encoding='utf-8') as f:
    long_description = f.read()

setup(
    name='smalltools',
    version='0.1.3',
    description='Small utilities for sysadmins',
    long_description=long_description,
    url='https://github.com/vivo75/smalltools',
    author='Francesco Riosa',
    author_email='vivo75+smalltools@gmail.com',
    include_package_data=True,

    # For a list of valid classifiers, see
    # https://pypi.python.org/pypi?%3Aaction=list_classifiers
    classifiers=[  # Optional
        'Development Status :: 4 - Beta',

        # Indicate who your project is intended for
        'Intended Audience :: Developers',
        'Topic :: System :: Systems Administration',

        # Pick your license as you wish (should match "license" above)
        'License :: OSI Approved :: GNU General Public License v2 or later (GPLv2+)',

        'Programming Language :: Python :: 3.5',
        'Programming Language :: Python :: 3.6',
        'Programming Language :: Python :: 3.7',
        'Programming Language :: Unix Shell',
    ],

    # Note that this is a string of words separated by whitespace, not a list.
    keywords='management logging helpers',

    packages=['stlog',],
    package_dir = {'': 'src'},

    entry_points={  # Optional
        'console_scripts': [
            'stlog = stlog:main',
        ],
    },
    data_files = [
        ('/etc/st-zfs-pull-snap.d', ['etc/st-zfs-pull-snap.d/conf1.pool.example']),
    ],
    scripts = [
        'bash/st-zfs-pull-snap'
    ],
)

# kate: encoding utf-8; eol unix; syntax Python;
# kate: indent-width 4; mixedindent off; replace-tabs on;
# kate: remove-trailing-space on; space-indent on;
# kate: word-wrap-column 500; word-wrap off;
