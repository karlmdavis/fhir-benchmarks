# [IBM FHIR Server](https://github.com/IBM/FHIR)

The [IBM FHIR Server](https://github.com/IBM/FHIR)
  is an open source FHIR server.
It is written in Java and supports a number of backend databases:
  Derby, PostgreSQL, and DB2.
Here, we run it with PostgreSQL.

To start the server, use the provided shell script:

    $ ./ibm_fhir_launch.sh

To stop the server, Docker Compose can be used as normal:

    $ docker-compose --down

The whole setup here is a bit convoluted, but the inspiration for it comes from the IBM FHIR repo itself:
<https://github.com/IBM/FHIR/blob/main/build/docker/docker-compose.yml>.