package com.tokera.ate.delegates;

import com.esotericsoftware.kryo.Kryo;
import com.esotericsoftware.kryo.Serializer;
import com.esotericsoftware.kryo.io.Input;
import com.esotericsoftware.kryo.io.Output;
import com.esotericsoftware.kryo.serializers.CompatibleFieldSerializer;
import com.esotericsoftware.kryo.serializers.DefaultSerializers;
import com.esotericsoftware.kryo.util.Pool;
import de.javakaffee.kryoserializers.*;
import org.checkerframework.checker.nullness.qual.NonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;
import java.lang.reflect.InvocationHandler;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.net.URI;
import java.util.*;
import java.util.regex.Pattern;

@ApplicationScoped
public class ObjectSerializerDelegate {

    private final ThreadLocal<@Nullable Kryo> kryos = ThreadLocal.withInitial(() -> createKryo());

    private final Pool<Kryo> kryoPool = new Pool<Kryo>(true, true) {
        protected Kryo create() { return ObjectSerializerDelegate.createKryo(); }
    };
    private final Pool<Output> outputPool = new Pool<Output>(true, true) {
        protected Output create () {
            return new Output(1024, -1);
        }
    };

    public static Kryo createKryo() {

        Kryo kryo = new KryoReflectionFactorySupport() {

            @Override
            @SuppressWarnings( { "rawtypes", "unchecked" } )
            public Serializer<?> getDefaultSerializer(final Class type ) {
                if ( EnumSet.class.isAssignableFrom( type ) ) {
                    return new EnumSetSerializer();
                }
                if ( EnumMap.class.isAssignableFrom( type ) ) {
                    return new EnumMapSerializer();
                }
                if ( Collection.class.isAssignableFrom( type ) ) {
                    return new CopyForIterateCollectionSerializer();
                }
                if ( Map.class.isAssignableFrom( type ) ) {
                    return new CopyForIterateMapSerializer();
                }
                if ( Date.class.isAssignableFrom( type ) ) {
                    return new DateSerializer( type );
                }
                return super.getDefaultSerializer( type );
            }
        };
        kryo.setReferences(false);
        kryo.setCopyReferences(false);
        kryo.setRegistrationRequired(false);
        kryo.register( Arrays.asList( "" ).getClass(), new ArraysAsListSerializer() );
        kryo.register( Collections.EMPTY_LIST.getClass(), new CollectionsEmptyListSerializer() );
        kryo.register( Collections.EMPTY_MAP.getClass(), new CollectionsEmptyMapSerializer() );
        kryo.register( Collections.EMPTY_SET.getClass(), new CollectionsEmptySetSerializer() );
        kryo.register( Collections.singletonList( "" ).getClass(), new CollectionsSingletonListSerializer() );
        kryo.register( Collections.singleton( "" ).getClass(), new CollectionsSingletonSetSerializer() );
        kryo.register( Collections.singletonMap( "", "" ).getClass(), new CollectionsSingletonMapSerializer() );
        kryo.register( BigDecimal.class, new DefaultSerializers.BigDecimalSerializer() );
        kryo.register( BigInteger.class, new DefaultSerializers.BigIntegerSerializer() );
        kryo.register( Pattern.class, new RegexSerializer() );
        kryo.register( BitSet.class, new BitSetSerializer() );
        kryo.register( URI.class, new URISerializer() );
        kryo.register( UUID.class, new UUIDSerializer() );
        kryo.register( GregorianCalendar.class, new GregorianCalendarSerializer() );
        kryo.register( InvocationHandler.class, new JdkProxySerializer() );
        UnmodifiableCollectionsSerializer.registerSerializers( kryo );
        SynchronizedCollectionsSerializer.registerSerializers( kryo );

        for (Class<?> clazz : AteDelegate.get().yamlDiscovery.getYamlTagClasses()) {
            CompatibleFieldSerializer.CompatibleFieldSerializerConfig config = new CompatibleFieldSerializer.CompatibleFieldSerializerConfig();
            config.setChunkedEncoding(false);
            config.setReadUnknownTagData(false);

            if (clazz.isEnum()) {
                kryo.register(clazz);
            } else {
                CompatibleFieldSerializer serializer = new CompatibleFieldSerializer(kryo, clazz, config);
                kryo.register(clazz, serializer);
            }
        }
        return kryo;
    }

    public byte[] serializeObj(@NonNull Object obj)
    {
        synchronized (this) {
            Kryo kryo = kryoPool.obtain();
            Output output = outputPool.obtain();

            kryo.writeClassAndObject(output, obj);
            byte[] ret = output.toBytes();

            outputPool.free(output);
            kryoPool.free(kryo);
            return ret;
        }
    }

    public Object deserializeObj(byte[] bytes)
    {
        synchronized (this) {
            Kryo kryo = kryoPool.obtain();
            Input input = new Input(bytes);

            Object ret = kryo.readClassAndObject(input);

            kryoPool.free(kryo);
            return ret;
        }
    }
}
